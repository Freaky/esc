/// Email Sucks Completely / Email Search Command

extern crate mailparse;
extern crate tantivy;
extern crate walkdir;
#[macro_use]
extern crate crossbeam_channel;

use mailparse::*;

use tantivy::collector::TopCollector;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::Index;

use walkdir::WalkDir;

use crossbeam_channel as channel;

use std::time::Instant;
use std::fs;
use std::path::Path;

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
    let index = open_search_index(INDEX_LOCATION);
    let schema = index.schema();

    let mut index_writer = index.writer_with_num_threads(4, 250_000_000).expect("index writer");

    let id = schema.get_field("id").expect("id");
    let path = schema.get_field("path").expect("path");
    // let date = schema.get_field("date").unwrap();
    let subject = schema.get_field("subject").expect("subject");
    let body = schema.get_field("body").expect("body");

    let mut indexed = 0;
    let start = Instant::now();

    for dir in dirs.iter() {
        let walker = WalkDir::new(dir).min_depth(3).max_depth(3).into_iter();
        for entry in walker {
            if let Ok(entry) = entry {
                if indexed % 10000 == 0 {
                    let elapsed = start.elapsed();
                    println!(
                        "[{} {:.2}/sec] {:?} {}",
                        indexed,
                        indexed as f64 / (elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 * 1e-9),
                        start.elapsed(),
                        entry.path().display()
                    );
                }
                if let Ok(message) = fs::read(&entry.path()) {
                    if let Ok(email) = parse_mail(&message) {
                        let m_id = email.headers.get_first_value("Message-Id");
                        let m_sub = email.headers.get_first_value("Subject");
                        let m_body = email.get_body();

                        if let (Ok(Some(m_id)), Ok(Some(m_sub)), Ok(m_body)) = (m_id, m_sub, m_body)
                        {
                            let mut doc = Document::default();
                            doc.add_text(path, &entry.path().to_string_lossy());
                            doc.add_text(id, &m_id);
                            doc.add_text(subject, &m_sub);
                            doc.add_text(body, &m_body);

                            index_writer.add_document(doc);
                            indexed += 1;
                        }
                    }
                }
            }
        }
    }

    index_writer.commit().expect("commit");
    println!("Indexed {} messages in {:?}", indexed, start.elapsed());

    index_writer.wait_merging_threads().unwrap();
    println!("Final merge finished after {:?}", start.elapsed());
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
    /*    index_emails(
        &["/home/freaky/Maildir/.unfiltered",
        "/home/freaky/Maildir/.archive.2016.unfiltered",
        "/home/freaky/Maildir/.archive.2015.unfiltered",
        "/home/freaky/Maildir/.archive.2014.unfiltered",
        "/home/freaky/Maildir/.archive.2013.unfiltered",
        "/home/freaky/Maildir/.archive.2012.unfiltered",
        "/home/freaky/Maildir/.archive.2011.unfiltered",
        "/home/freaky/Maildir/.archive.2010.unfiltered"]
    );*/
    index_emails(&["/home/freaky/Maildir/"]);
    // index_emails(&["/home/freaky/Maildir/.spam.high"]);
    search("freshbsd v4 exception");
}
