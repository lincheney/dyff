// use assert_cmd::prelude::*; // Add methods on commands
// use predicates::prelude::*; // Used for writing assertions
use std::process::Command; // Run programs

fn run_test(file: String) {
    let output = Command::new("bash")
        .arg("-c")
        .arg(format!("< fixtures/input/{} cargo run -- --color=always --inline=always", file))
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());

    let expected = std::fs::read_to_string(format!("fixtures/output/{}", file)).unwrap();
    assert_eq!(std::str::from_utf8(&output.stdout).unwrap(), expected);
}

macro_rules! fixture_test {
    ($file:ident) => {
        #[test]
        fn $file() {
            run_test(stringify!($file).to_owned())
        }
    }
}

fixture_test!(diff1);
fixture_test!(diff2);
fixture_test!(diff3);
fixture_test!(diff4);
fixture_test!(diff5);
fixture_test!(diff6);
fixture_test!(diff7);
fixture_test!(diff8);
fixture_test!(diff9);
fixture_test!(diff10);
fixture_test!(diff11);
fixture_test!(diff12);
fixture_test!(diff13);
fixture_test!(diff14);
fixture_test!(diff15);
fixture_test!(diff16);
fixture_test!(diff17);
fixture_test!(diff18);
fixture_test!(diff19);
fixture_test!(diff20);
fixture_test!(diff21);
fixture_test!(diff22);
fixture_test!(diff23);
fixture_test!(diff24);
fixture_test!(diff25);
fixture_test!(diff26);
fixture_test!(diff27);
fixture_test!(diff28);
fixture_test!(diff29);
fixture_test!(diff30);
