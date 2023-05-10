use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

use postgres::{Client, NoTls};

use crate::model::user::User;
use crate::util::{get_id, get_user_request_body};
use crate::{INTERNAL_SERVER_ERROR, NOT_FOUND, OK_RESPONSE};

pub trait StreamHandler {
    fn handle(&self, stream: &mut TcpStream) -> ();
}
pub struct Server {
    port: String,
    hostname: String,
    handler: Box<dyn StreamHandler>,
}

impl Server {
    pub fn new(hostname: String, port: String, handler: Box<dyn StreamHandler>) -> Self {
        Self {
            hostname,
            port,
            handler,
        }
    }

    pub fn listen(&self) {
        //start server and print port
        let listener = TcpListener::bind(format!("{}:{}", self.hostname, self.port)).unwrap();
        println!("Server listening on port {} ", self.port);

        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => self.handler.handle(&mut stream),
                Err(e) => {
                    println!("Unable to connect: {}", e);
                }
            }
        }
    }
}

pub struct Handler {
    database_url: String,
}
impl StreamHandler for Handler {
    fn handle(&self, stream: &mut TcpStream) -> () {
        let mut buffer = [0; 1024];
        let mut request = String::new();

        match stream.read(&mut buffer) {
            Ok(size) => {
                request.push_str(String::from_utf8_lossy(&buffer[..size]).as_ref());

                let (status_line, content) = match &*request {
                    r if r.starts_with("POST /users") => self.handle_post_request(r),
                    r if r.starts_with("GET /users/") => self.handle_get_request(r),
                    r if r.starts_with("GET /users") => self.handle_get_all_request(r),
                    r if r.starts_with("PUT /users/") => self.handle_put_request(r),
                    r if r.starts_with("DELETE /users/") => self.handle_delete_request(r),
                    _ => (NOT_FOUND.to_string(), "404 not found".to_string()),
                };

                stream
                    .write_all(format!("{}{}", status_line, content).as_bytes())
                    .unwrap();
            }
            Err(e) => eprintln!("Unable to read stream: {}", e),
        }
    }
}

impl Handler {
    pub fn new(database_url: String) -> Self {
        Self { database_url }
    }
    //CONTROLLERS

    //handle_post_request function
    fn handle_post_request(&self, request: &str) -> (String, String) {
        match (
            get_user_request_body(&request),
            Client::connect(self.database_url.as_str(), NoTls),
        ) {
            (Ok(user), Ok(mut client)) => {
                client
                    .execute(
                        "INSERT INTO users (name, email) VALUES ($1, $2)",
                        &[&user.name, &user.email],
                    )
                    .unwrap();

                (OK_RESPONSE.to_string(), "User created".to_string())
            }
            _ => (INTERNAL_SERVER_ERROR.to_string(), "Error".to_string()),
        }
    }

    //handle_get_request function
    fn handle_get_request(&self, request: &str) -> (String, String) {
        match (
            get_id(&request).parse::<i32>(),
            Client::connect(self.database_url.as_str(), NoTls),
        ) {
            (Ok(id), Ok(mut client)) => {
                match client.query_one("SELECT * FROM users WHERE id = $1", &[&id]) {
                    Ok(row) => {
                        let user = User {
                            id: row.get(0),
                            name: row.get(1),
                            email: row.get(2),
                        };

                        (
                            OK_RESPONSE.to_string(),
                            serde_json::to_string(&user).unwrap(),
                        )
                    }
                    _ => (NOT_FOUND.to_string(), "User not found".to_string()),
                }
            }

            _ => (INTERNAL_SERVER_ERROR.to_string(), "Error".to_string()),
        }
    }

    //handle_get_all_request function
    fn handle_get_all_request(&self, request: &str) -> (String, String) {
        match Client::connect(self.database_url.as_str(), NoTls) {
            Ok(mut client) => {
                let mut users = Vec::new();

                for row in client.query("SELECT * FROM users", &[]).unwrap() {
                    users.push(User {
                        id: row.get(0),
                        name: row.get(1),
                        email: row.get(2),
                    });
                }

                (
                    OK_RESPONSE.to_string(),
                    serde_json::to_string(&users).unwrap(),
                )
            }
            _ => (INTERNAL_SERVER_ERROR.to_string(), "Error".to_string()),
        }
    }

    //handle_put_request function
    fn handle_put_request(&self, request: &str) -> (String, String) {
        match (
            get_id(&request).parse::<i32>(),
            get_user_request_body(&request),
            Client::connect(self.database_url.as_str(), NoTls),
        ) {
            (Ok(id), Ok(user), Ok(mut client)) => {
                client
                    .execute(
                        "UPDATE users SET name = $1, email = $2 WHERE id = $3",
                        &[&user.name, &user.email, &id],
                    )
                    .unwrap();

                (OK_RESPONSE.to_string(), "User updated".to_string())
            }
            _ => (INTERNAL_SERVER_ERROR.to_string(), "Error".to_string()),
        }
    }

    //handle_delete_request function
    fn handle_delete_request(&self, request: &str) -> (String, String) {
        match (
            get_id(&request).parse::<i32>(),
            Client::connect(self.database_url.as_str(), NoTls),
        ) {
            (Ok(id), Ok(mut client)) => {
                let rows_affected = client
                    .execute("DELETE FROM users WHERE id = $1", &[&id])
                    .unwrap();

                if rows_affected == 0 {
                    return (NOT_FOUND.to_string(), "User not found".to_string());
                }

                (OK_RESPONSE.to_string(), "User deleted".to_string())
            }
            _ => (INTERNAL_SERVER_ERROR.to_string(), "Error".to_string()),
        }
    }
}
