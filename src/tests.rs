use std::time::Duration;

#[cfg(test)]
use crate::SystemHttpClientInterface;

const TIMEOUT: Option<Duration> = Some(Duration::from_secs(30));

#[test]
fn test_get() {
	let reqwest = reqwest::blocking::Client::new();

	for test_url in ["https://www.google.com/favicon.ico", "http://www.example.org"] {
		for client in crate::clients::all_http_clients() {
			let result = client.get(test_url, TIMEOUT).unwrap();

			let example = reqwest.get(test_url).send().unwrap().bytes().unwrap();

			if example != result.body {
				let example = String::from_utf8_lossy(&example);
				let result = String::from_utf8_lossy(&result.body);
				panic!(
					"Client: {client:?}\nURL: {test_url}\n\nDiff:\n{}",
					difference::Changeset::new(example.as_ref(), result.as_ref(), "")
				);
			}
		}
	}
}

#[test]
fn test_naughty_url() {
	match super::get("file:///etc/passwd") {
		Ok(_) => panic!("pwned"),
		Err(super::Error::InvalidUrlScheme) => {}
		Err(err) => panic!("{err}"),
	}
}

#[test]
fn test_str_interp_url() {
	std::env::set_var("SYSREQ_PWNED", "http://example.org");

	for client in crate::clients::all_http_clients() {
		for interp in ["$SYSREQ_PWNED", "`SYSREQ_PWNED`", "${SYSREQ_PWNED}", "[[SYSREQ_PWNED]]"].into_iter() {
			if let Ok(result) = client.get(interp, TIMEOUT) {
				if !result.body.is_empty() {
					panic!("This should have failed: {}", String::from_utf8_lossy(&result.body));
				}
			}
		}

		if let Ok(result) = client.get("#//\"\"\"\"\"'''''[[]]`````${hello}$hello###", TIMEOUT) {
			if !result.body.is_empty() {
				panic!("This should have failed: {}", String::from_utf8_lossy(&result.body));
			}
		}

		let example = client.get("http://example.org", TIMEOUT).unwrap();
		let result = client
			.get("http://example.org/#//\"\"\"\"\"'''''[[]]`````${hello}$hello###", TIMEOUT)
			.unwrap();
		if example.body != result.body {
			let example = String::from_utf8_lossy(&example.body);
			let result = String::from_utf8_lossy(&result.body);
			panic!("Diff:\n{}", difference::Changeset::new(example.as_ref(), result.as_ref(), ""));
		}
	}
}

#[test]
#[should_panic]
fn test_timeout_zero() {
	let _ = crate::RequestBuilder::new("http://localhost").timeout(Some(Duration::ZERO));
}

#[test]
fn test_timeouts() {
	let (tx, rx) = std::sync::mpsc::sync_channel(1);

	std::thread::spawn(move || {
		let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();

		tx.send(server.local_addr().unwrap()).unwrap();

		for client in server.incoming() {
			std::mem::forget(client);
		}
	});

	let url = format!("http://{}", rx.recv().unwrap());

	println!("Testing timeouts on {url}");

	for client in crate::clients::all_http_clients() {
		println!("Testing timeout for client {client:?}");
		let result = client.get(&url, Some(Duration::from_secs_f64(0.1)));
		match result {
			Err(crate::Error::IoError(err)) if err.kind() == std::io::ErrorKind::TimedOut => {}
			_ => panic!("{client:?}: {result:?}"),
		}
	}
}
