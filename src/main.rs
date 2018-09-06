/// Email Sucks Completely / Email Search Command
extern crate mailparse;
#[macro_use]
extern crate tantivy;
extern crate walkdir;
#[macro_use]
extern crate crossbeam;

#[macro_use]
extern crate structopt;

use mailparse::*;

use tantivy::collector::TopCollector;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::Index;

use walkdir::WalkDir;

use crossbeam::channel;

use structopt::StructOpt;

use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::Instant;

const INDEX_DIRECTORY: &str = "/tmp/email_sucks_completely/";

#[derive(Debug, StructOpt)]
struct IndexOptions {
    /// Email read/parse thread count
    #[structopt(long = "read-threads", default_value = "2")]
    read_threads: usize,

    /// Tantivy index thread count
    #[structopt(long = "index-threads", default_value = "1")]
    index_threads: usize,

    /// Tantivy index buffer size in MB
    #[structopt(long = "index-buffer", default_value = "256")]
    index_buffer: usize,

    /// Maildir base directory to index
    #[structopt(parse(from_os_str))]
    dirs: Vec<PathBuf>,
}

#[derive(Debug, StructOpt)]
#[structopt(name = "esc", about = "Email Search Command")]
struct EscArgs {
    /// Directory for Tantivy search index
    #[structopt(short = "d", long = "index-dir", parse(from_os_str))]
    index_dir: Option<PathBuf>,

    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    #[structopt(name = "index")]
    Index(IndexOptions),
    #[structopt(name = "search")]
    Search { query: String },
}

struct Esc {
    dir: PathBuf
}

impl Esc {
    fn new<P: Into<PathBuf>>(dir: P) -> Self {
        Self { dir: dir.into() }
    }

    fn open(&mut self) -> Index {
        if let Ok(index) = Index::open_in_dir(&self.dir) {
            index
        } else {
            let mut schema_builder = SchemaBuilder::default();
            schema_builder.add_text_field("id", STRING | STORED);
            schema_builder.add_text_field("path", STRING | STORED);
            // schema_builder.add_i64_field("date", INT_INDEXED);
            schema_builder.add_text_field("subject", TEXT | STORED);
            schema_builder.add_text_field("body", TEXT);
            let schema = schema_builder.build();

            let _ = fs::create_dir_all(&self.dir);
            Index::create_in_dir(&self.dir, schema).expect("create index")
        }
    }

    fn index(&mut self, opts: &IndexOptions) {
        let (send_file, recv_file) = channel::bounded::<walkdir::DirEntry>(128);
        let (send_idx, recv_idx) = channel::bounded::<Document>(16);

        let start = Instant::now();
        let index = self.open();
        let mut index_writer = index
            .writer_with_num_threads(opts.index_threads, opts.index_buffer * 1024 * 1024)
            .expect("index writer");

        let schema = index.schema();
        let id = schema.get_field("id").expect("id");
        let path = schema.get_field("path").expect("path");
        let subject = schema.get_field("subject").expect("subject");
        let body = schema.get_field("body").expect("body");

        crossbeam::scope(|scope| {
            // WalkDir thread, -> send_file
            scope.spawn(move || {
                for dir in &opts.dirs {
                    let walker = WalkDir::new(dir).min_depth(3).max_depth(3).into_iter();

                    for entry in walker {
                        entry.map(|entry| send_file.send(entry)).ok();
                    }
                }

                drop(send_file);
            });

            // Mail parse thread, recv_file -> send_idx, multiple
            for _ in 0..opts.read_threads {
                let recv_file = recv_file.clone();
                let send_idx = send_idx.clone();
                scope.spawn(move || {
                    for entry in recv_file {
                        if let Ok(attr) = entry.metadata() {
                            if !(attr.is_file() && attr.len() < 1024 * 1024 * 4) {
                                continue;
                            }
                            fs::read(&entry.path())
                                .and_then(|message| {
                                    let email = parse_mail(&message).map_err(|_| {
                                        io::Error::new(io::ErrorKind::Other, "Failed parsing email")
                                    })?;

                                    let m_id = email.headers.get_first_value("Message-Id");
                                    let m_sub = email.headers.get_first_value("Subject");
                                    let m_body = email.get_body();

                                    if let (Ok(Some(m_id)), Ok(Some(m_sub)), Ok(m_body)) =
                                        (m_id, m_sub, m_body)
                                    {
                                        let doc = doc!(
                                        path => entry.path().to_string_lossy().to_string(),
                                        id => m_id,
                                        subject => m_sub,
                                        body => m_body
                                    );
                                        send_idx.send(doc);
                                    };
                                    Ok(())
                                }).ok();
                        }
                    }

                    drop(send_idx);
                });
            }
            drop(send_idx);

            // Index thread, recv_idx -> tantivy
            scope.spawn(|| {
                let mut indexed = 0;

                for doc in recv_idx {
                    index_writer.add_document(doc);
                    indexed += 1;

                    if indexed % 10000 == 0 {
                        let elapsed = start.elapsed();
                        println!(
                            "[{} {:.2}/sec {:?}]",
                            indexed,
                            indexed as f64
                                / (elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 * 1e-9),
                            elapsed
                        );
                    }
                }

                index_writer.commit().expect("commit");
                println!("Indexed {} messages in {:?}", indexed, start.elapsed());

                index_writer.wait_merging_threads().unwrap();
                println!("Final merge finished after {:?}", start.elapsed());
            });
        });
    }

    fn search(&mut self, query: &str) {
        let start = Instant::now();

        let index = self.open();
        let schema = index.schema();

        let path = schema.get_field("path").expect("path");
        let subject = schema.get_field("subject").expect("subject");
        let body = schema.get_field("body").expect("body");

        index.load_searchers().expect("load_searchers");
        let searcher = index.searcher();

        let query_parser = QueryParser::for_index(&index, vec![subject, body]);
        let query = query_parser.parse_query(query).expect("parse query");

        let mut top_collector = TopCollector::with_limit(25);
        searcher.search(&*query, &mut top_collector).unwrap();
        let doc_addresses = top_collector.docs();
        for doc_address in doc_addresses {
            let retrieved_doc = searcher.doc(&doc_address).unwrap();
            println!(
                "{}: {}",
                retrieved_doc.get_first(path).unwrap().text(),
                retrieved_doc.get_first(subject).unwrap().text()
            );
        }

        println!("searched in {:?}", start.elapsed());
    }
}

fn main() {
    let opts = EscArgs::from_args();
    let index = opts
        .index_dir
        .unwrap_or_else(|| PathBuf::from(INDEX_DIRECTORY));

    let mut esc = Esc::new(index);
    match opts.cmd {
        Command::Index(index_opts) => {
            esc.index(&index_opts);
        }
        Command::Search { query } => {
            esc.search(&query);
        }
    }
}
