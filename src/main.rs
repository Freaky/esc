/// Email Sucks Completely / Email Search Command

extern crate mailparse;
#[macro_use]
extern crate tantivy;
extern crate walkdir;
#[macro_use]
extern crate crossbeam;

use mailparse::*;

use tantivy::collector::TopCollector;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::Index;

use walkdir::WalkDir;

use crossbeam::channel as channel;

use std::time::Instant;
use std::fs;
use std::path::Path;
use std::io;

const INDEX_LOCATION: &str = "/tmp/email_sucks_completely/";

fn open_search_index<P: AsRef<Path>>(index_dir: P) -> Index {
    let index_dir = index_dir.as_ref();

    if let Ok(index) = Index::open_in_dir(index_dir) {
        return index;
    } else {
        let mut schema_builder = SchemaBuilder::default();
        schema_builder.add_text_field("id", STRING | STORED);
        schema_builder.add_text_field("path", STRING | STORED);
        // schema_builder.add_i64_field("date", INT_INDEXED);
        schema_builder.add_text_field("subject", TEXT | STORED);
        schema_builder.add_text_field("body", TEXT);
        let schema = schema_builder.build();

        return Index::create_in_dir(index_dir, schema).expect("create index");
    }
}

fn index_emails(dirs: &[&str]) {
    let (send_file, recv_file) = channel::bounded::<walkdir::DirEntry>(128);
    let (send_idx, recv_idx) = channel::bounded::<Document>(16);

    let start = Instant::now();
    let index = open_search_index(INDEX_LOCATION);
    let mut index_writer = index.writer_with_num_threads(4, 500_000_000).expect("index writer");

    let schema = index.schema();
    let id = schema.get_field("id").expect("id");
    let path = schema.get_field("path").expect("path");
    let subject = schema.get_field("subject").expect("subject");
    let body = schema.get_field("body").expect("body");

    crossbeam::scope(|scope| {
        // Index thread, recv_doc -> tantivy
        scope.spawn(|| {
            let mut indexed = 0;

            for doc in recv_idx {
                index_writer.add_document(doc);
                indexed += 1;

                if indexed % 10000 == 0 {
                    let elapsed = start.elapsed();
                    println!(
                        "[{} {:.2}/sec] {:?}",
                        indexed,
                        indexed as f64 / (elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 * 1e-9),
                        elapsed
                    );
                }
            }

            index_writer.commit().expect("commit");
            println!("Indexed {} messages in {:?}", indexed, start.elapsed());

            index_writer.wait_merging_threads().unwrap();
            println!("Final merge finished after {:?}", start.elapsed());
        });

        // Mail parse thread, recv_file -> send_doc, multiple
        for _ in 0..8 {
            let my_recv_file = recv_file.clone();
            let my_send_idx = send_idx.clone();
            scope.spawn(move || {
                for entry in my_recv_file {
                    if let Ok(attr) = entry.metadata() {
                        if !(attr.is_file() && attr.len() < 1024 * 1024 * 4) {
                            continue;
                        }
                        fs::read(&entry.path()).and_then(|message| {
                            parse_mail(&message).and_then(|email|
                            {
                                let m_id = email.headers.get_first_value("Message-Id");
                                let m_sub = email.headers.get_first_value("Subject");
                                let m_body = email.get_body();

                                if let (Ok(Some(m_id)), Ok(Some(m_sub)), Ok(m_body)) = (m_id, m_sub, m_body) {
                                    let doc = doc!(
                                        path => entry.path().to_string_lossy().to_string(),
                                        id => m_id,
                                        subject => m_sub,
                                        body => m_body
                                    );
                                    my_send_idx.send(doc);
                                }
                                Ok(())
                            }).map_err(|_| io::Error::new(io::ErrorKind::Other, "Failed parsing email"))
                        }).ok();
                    }
                }

                drop(my_send_idx);
            });
        }

        drop(send_idx);

        // WalkDir thread, -> send_file
        scope.spawn(move || {
            for dir in dirs.iter() {
                let walker = WalkDir::new(dir).min_depth(3).max_depth(3).into_iter();

                for entry in walker {
                    entry.and_then(|entry| Ok(send_file.send(entry))).ok();
                }
            }

            drop(send_file);
        });
    });
}

fn search(query: &str) {
    let start = Instant::now();

    let index = open_search_index(INDEX_LOCATION);
    let schema = index.schema();

    let path = schema.get_field("path").expect("path");
    let subject = schema.get_field("subject").expect("subject");
    let body = schema.get_field("body").expect("body");

    index.load_searchers().expect("load_searchers");
    let searcher = index.searcher();

    let query_parser = QueryParser::for_index(&index, vec![subject, body]);
    let query = query_parser.parse_query(query).expect("parse query");

    let mut top_collector = TopCollector::with_limit(10);
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

fn main() {
    index_emails(&["/home/freaky/Maildir/"]);
    search("freshbsd v4 exception");
}
