use std::env;

use rust_rest::{
    server::{Handler, Server},
    util::set_database,
};

#[macro_use]
extern crate serde_derive;

fn main() {
    let database_url = match env::var("DATABASE_URL") {
        Ok(v) => v,
        Err(e) => panic!("Database URL is not set {} ", e),
    };

    //bootstrap
    set_database(&database_url).unwrap();
    let handler = Handler::new(database_url);
    let server_handler = Box::new(handler);
    let server = Server::new(
        String::from("0.0.0.0"),
        String::from("8081"),
        server_handler,
    );

    server.listen();

    println!("Hello, world!");
}
