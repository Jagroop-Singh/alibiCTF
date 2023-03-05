use askama::Template;
use axum::{
    body::Body,
    extract::{Path, Query},
    http::{
        header::{self, HeaderMap, HeaderName},
        HeaderValue, Method, Request, Response, StatusCode,
    },
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use axum_macros;
use serde::{de, Deserialize, Deserializer, Serialize};
use sqlite::State;
use std::{fmt, net::SocketAddr, str::FromStr};
use tower::util::ServiceExt;
use tower_http::cors::CorsLayer;

#[derive(Template)] // this will generate the code...
#[template(path = "hello.html")] // Using the template in this path, relative to the `templates` dir in the crate root
struct HelloTemplate<'a> {
    name: &'a str, // the field name should match the variable name
                   // in your template
}

#[derive(Template)]
#[template(path = "index.html")]
struct HomeTemplate<'a> {
    challenges: Vec<Challenge<'a>>,
}

#[derive(Template)]
#[template(path = "sql1.html")]
struct SqlTemplate {
    items: Vec<Item>,
}

struct Item {
    flight: String,
    tail_number: String,
    long: String,
    lat: String,
    manufacturer: String,
}

#[derive(Debug, Deserialize)]
struct ItemsQuery {
    #[serde(default, deserialize_with = "empty_string_as_none")]
    search: Option<String>,
}

/// Serde deserialization decorator to map empty Strings to None,
fn empty_string_as_none<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    T::Err: fmt::Display,
{
    let opt = Option::<String>::deserialize(de)?;
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => FromStr::from_str(s).map_err(de::Error::custom).map(Some),
    }
}

impl std::fmt::Display for Item {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            fmt,
            "{} {} {} {} {}",
            self.flight, self.tail_number, self.long, self.lat, self.manufacturer
        )
    }
}

struct Challenge<'a> {
    title: &'a str,
    description: &'a str,
    href: &'a str,
}

#[tokio::main]
async fn main() {
    // let hello = HelloTemplate { name: "world" }; //instantiate your struct
    // println!("{}", hello.render().unwrap()); // then render it.
    // intialize tracing
    tracing_subscriber::fmt::init();

    // build our application with a route
    let app = Router::new()
        // `Get /` goes to `root`
        .route("/", get(root))
        // `Get /hello` goes to `hello`
        .route("/hello", get(hello))
        // `Post /users` goes to `create_user`
        .route("/users", post(create_user))
        .route("/sqli/one", get(sql))
        .route("/static/sql1.css", get(get_css2))
        .route("/static/index.css", get(get_css));
    let app = app.fallback(handler_404);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    tracing::debug!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn handler_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "nothing to see here")
}
async fn sql(query: Query<ItemsQuery>) -> Html<String> {
    // Testing out Queries
    let item_queries: ItemsQuery = query.0;
    println!("{:?}", item_queries.search);

    let mut v: Vec<Item> = Vec::new();
    let connection = sqlite::open("challenge.db").unwrap();

    // let mut query: String = String::new();
    let query = match item_queries.search {
        Some(k) => {
            "Select * from local_flight_data ".to_owned()
                + &"where manufacturer LIKE '%"
                + &k
                + "%';"
        }
        None => "Select * from local_flight_data".to_string(),
    };
    println!("query: {}", query);

    let statement = connection.prepare(query);
    if let Err(e) = statement {
        return Html("DB Error: ".to_string() + &e.to_string());
    }
    let mut statement = statement.unwrap();
    let c = (|a: sqlite::Error|a.to_string());
    while let Ok(State::Row) = statement.next() {
        let flight = statement.read::<String, _>("flight").unwrap_or_else(c);
        let tail_number = statement.read::<String, _>("tail_number").unwrap_or_else(c);
        let long = statement.read::<String, _>("long").unwrap_or_else(c);
        let lat = statement.read::<String, _>("lat").unwrap_or_else(c);
        let manufacturer = statement.read::<String, _>("manufacturer").unwrap_or_else(c);
        println!(
            "flight: {}, tail: {}, long: {}, lat: {}, manufacturer: {}",
            flight, tail_number, long, lat, manufacturer
        );
        v.push(Item {
            flight: flight.to_owned(),
            tail_number: tail_number,
            long: long,
            lat: lat,
            manufacturer: manufacturer,
        });
    }
    let s = SqlTemplate { items: v };
    Html(s.render().unwrap())
}

// basic handler that response with a static string
async fn root() -> Html<String> {
    println!("visited root");
    let challenges: Vec<Challenge> = vec![
        Challenge {
            title: "SQLi One",
            description: "Your first go",
            href: "/sqli/one",
        },
        Challenge {
            title: "SQLi Two",
            description: "Your second go",
            href: "/",
        },
        Challenge {
            title: "SQLi Three",
            description: "Your third go",
            href: "/",
        },
        Challenge {
            title: "SQLi Four",
            description: "Your fourth go",
            href: "/",
        },
        Challenge {
            title: "SQLi Five",
            description: "Your fifth go",
            href: "/",
        },
        Challenge {
            title: "SQLi Six",
            description: "Your sixth go",
            href: "/",
        },
    ];
    Html(
        HomeTemplate {
            challenges: challenges,
        }
        .render()
        .unwrap(),
    )
    // Html(std::include_str!("../static/index.html"))
}

async fn get_css() -> impl IntoResponse {
    (
        // set status code
        StatusCode::OK,
        // headers with an array
        [("SERVER", "axum"), ("Content-Type", "text/css")],
        std::include_str!("../templates/index.css"),
    )
}
async fn get_css2() -> impl IntoResponse {
    (
        // set status code
        StatusCode::OK,
        // headers with an array
        [("SERVER", "axum"), ("Content-Type", "text/css")],
        std::include_str!("../templates/sql1.css"),
    )
}

#[axum_macros::debug_handler]
async fn hello() -> Html<String> {
    Html(HelloTemplate { name: "world" }.render().unwrap())
}

async fn create_user(
    // this argument tells axum to parse the request body
    // as JSON into a `CreateUser` type
    Json(payload): Json<CreateUser>,
) -> impl IntoResponse {
    // Insert your application login here
    let user = User {
        id: 1337,
        username: payload.username,
    };

    // this will be converted into a JSON response
    // with a status code of `201 Created`
    (StatusCode::CREATED, Json(user))
}

// the input to our `create_user` handler
#[derive(Deserialize)]
struct CreateUser {
    username: String,
}

// the output of our `create_user` handler
#[derive(Serialize)]
struct User {
    id: u64,
    username: String,
}
