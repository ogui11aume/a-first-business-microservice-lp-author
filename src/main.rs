use std::net::SocketAddr;
use std::convert::Infallible;
use std::str::{self, FromStr};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Client, Method, Request, Response, StatusCode, Server};
use csv::Reader;
use wasmedge_hyper_rustls;
use wasmedge_rustls_api;


async fn root_handler(_req: Request<Body>) -> Result<Response<Body>, anyhow::Error> {
    Ok(Response::new(Body::from(
        "Try POSTing data to /find_rate such as: `curl localhost:8001/get_rate -XPOST -d '87640'` for new exercice example or with '0000' for 'Not Found'",
    )))
}

async fn find_rate_api_handler(req: Request<Body>) -> Result<Response<Body>, anyhow::Error> {
    // parse body url
    //let post_body = hyper::body::to_bytes(req.into_body()).await?;
    //let post_body_str = String::from(str::from_utf8(&post_body)?);
    //let uri = hyper::Uri::from_str(post_body_str.as_str())?;

    let uri = hyper::Uri::from_str("https://raw.githubusercontent.com/openfisca/openfisca-france/master/openfisca_france/assets/taxe_habitation/parametres_th_2022.csv")?;

    let https = wasmedge_hyper_rustls::connector::new_https_connector(
        wasmedge_rustls_api::ClientConfig::default(),
    );

    let client = Client::builder().build::<_, hyper::Body>(https);

    // TODO: make request
    let res = client.get(uri).await?;

    let body = hyper::body::to_bytes(res.into_body()).await?;

    Ok(Response::new(Body::from(body)))
}

async fn find_rate_csv_handler(req: Request<Body>) -> Result<Response<Body>, anyhow::Error> {
    let post_body = hyper::body::to_bytes(req.into_body()).await?;
    let mut rate = "0.08".to_string(); // default is 8%
    let mut found: bool = false;

    let rates_data: &[u8] = include_bytes!("rates_by_zipcode.csv");
    let mut rdr = Reader::from_reader(rates_data);
    for result in rdr.records() {
        let record = result?;
        // dbg!("{:?}", record.clone());
        if str::from_utf8(&post_body).unwrap().eq(&record[0]) {
            rate = record[1].to_string();
            found = true;
            break;
        } 
    }

    if found {
        Ok(Response::new(Body::from(rate)))
    } else {
        let mut not_found = Response::default();
        *not_found.status_mut() = StatusCode::NOT_FOUND;
        *not_found.body_mut() = Body::from("Zip Code Not Found");
        Ok(not_found)
    }
}


async fn not_found_handler(_req: Request<Body>) -> Result<Response<Body>, anyhow::Error> {
    let mut res = Response::default();
    *res.status_mut() = StatusCode::NOT_FOUND;
    *res.body_mut() = Body::from("Route Not Found");
    Ok(res)
}

/// This is our service handler. It receives a Request, routes on its
/// path, and returns a Future of a Response.
async fn handle_request(req: Request<Body>) -> Result<Response<Body>, anyhow::Error> {
    

    match (req.uri().path(), req.method()) {
        ("/", &Method::GET) => root_handler(req).await,
        ("/find_rate_api", &Method::POST) => find_rate_api_handler(req).await,
        ("/find_rate_csv", &Method::POST) => find_rate_csv_handler(req).await,
        _ => not_found_handler(req).await,
    }

}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([0, 0, 0, 0], 8001));
    let make_svc = make_service_fn(|_| {
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                handle_request(req)
            }))
        }
    });
    let server = Server::bind(&addr).serve(make_svc);
    dbg!("Server started on port 8001");
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
    Ok(())
}
