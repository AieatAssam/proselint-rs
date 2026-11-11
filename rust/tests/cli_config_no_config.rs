use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-no-config-default-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated workspace");
        for name in ["home", "xdg"] {
            std::fs::create_dir(path.join(name)).expect("create isolated config home");
        }
        Self(path)
    }

    fn run(&self, input: &[u8]) -> Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
            .current_dir(&self.0)
            .env("HOME", self.0.join("home"))
            .env("XDG_CONFIG_HOME", self.0.join("xdg"))
            .args(["check"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("start native CLI");
        child
            .stdin
            .take()
            .expect("piped stdin")
            .write_all(input)
            .expect("write stdin");
        child.wait_with_output().expect("wait for CLI")
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn no_config_found_uses_default_enabled_rule() {
    let output = Workspace::new().run(b"This is very good.");
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert_eq!(
        output.stdout,
        b"<stdin>:1:9: weasel_words.very: Substitute 'damn' every time you're inclined to write 'very'; your editor will delete it and the writing will be just as it should be.\n"
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn no_config_found_enables_annotations_misc() {
    let output = Workspace::new().run(b"TODO");
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert_eq!(
        output.stdout,
        b"<stdin>:1:1: annotations.misc: Annotation left in text.\n"
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn no_config_found_enables_archaism() {
    let output = Workspace::new().run(b"Alack, then plain text.");
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert_eq!(
        output.stdout,
        b"<stdin>:1:1: archaism: 'Alack' is archaic.\n"
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn no_config_found_enables_cliches_hell_and_hedging() {
    let cases = [
        (
            b"Then all hell broke loose again.".as_slice(),
            b"<stdin>:1:6: cliches.hell: Never use the phrase 'all hell broke loose'.\n".as_slice(),
        ),
        (
            b"I would argue that this is a good test.".as_slice(),
            b"<stdin>:1:1: hedging: Hedging. Just say it.\n".as_slice(),
        ),
    ];
    for (input, expected) in cases {
        let output = Workspace::new().run(input);
        assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
        assert_eq!(output.stdout, expected);
        assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    }
}

#[test]
fn no_config_found_enables_all_am_pm_checks() {
    let cases = [
        (
            b"Meet at 9am.".as_slice(),
            b"<stdin>:1:9: dates_times.am_pm.lowercase_periods: It's standard to use periods for lowercase 'a.m.' or 'p.m.'.\n<stdin>:1:9: dates_times.am_pm.spacing: It's standard to put a space before 'a.m.' or 'p.m.'.\n".as_slice(),
        ),
        (
            b"Meet at 9pm.".as_slice(),
            b"<stdin>:1:9: dates_times.am_pm.lowercase_periods: It's standard to use periods for lowercase 'a.m.' or 'p.m.'.\n<stdin>:1:9: dates_times.am_pm.spacing: It's standard to put a space before 'a.m.' or 'p.m.'.\n".as_slice(),
        ),
        (
            b"It starts at 12pm.".as_slice(),
            b"<stdin>:1:14: dates_times.am_pm.lowercase_periods: It's standard to use periods for lowercase 'a.m.' or 'p.m.'.\n<stdin>:1:14: dates_times.am_pm.spacing: It's standard to put a space before 'a.m.' or 'p.m.'.\n<stdin>:1:14: dates_times.am_pm.midnight_noon: '12 a.m.' and '12 p.m.' are ambiguous. Use 'midnight' or 'noon'.\n".as_slice(),
        ),
    ];
    for (input, expected) in cases {
        let output = Workspace::new().run(input);
        assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
        assert_eq!(output.stdout, expected);
        assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    }
}

#[test]
fn no_config_found_enables_social_awareness_preferred_forms() {
    let output = Workspace::new().run(b"Special rights should mean equal rights.");
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert_eq!(
        output.stdout,
        b"<stdin>:1:1: social_awareness.lgbtq.terms: Possibly offensive term. Consider using 'equal rights' instead of 'Special rights'.\n"
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn no_config_found_enables_animal_adjectives() {
    let output = Workspace::new().run(b"It was some bird-like creature.");
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert_eq!(
        output.stdout,
        b"<stdin>:1:13: terms.animal_adjectives: There's a word for this: 'avine'.\n"
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn no_config_found_enables_norris_denizen_labels() {
    let output = Workspace::new().run(b"Manchesterian");
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert_eq!(
        output.stdout,
        b"<stdin>:1:1: terms.denizen_labels.norris: Would you like 'Mancunian'?\n"
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn no_config_found_enables_garner_denizen_labels() {
    let output = Workspace::new().run(b"Hong Kongite");
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert_eq!(
        output.stdout,
        b"<stdin>:1:1: terms.denizen_labels.garner: 'Hong Konger' is the preferred denizen label.\n"
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn no_config_found_enables_nonwords() {
    let output = Workspace::new().run(b"affrontery");
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert_eq!(
        output.stdout,
        b"<stdin>:1:1: nonwords: Nonword - try 'effrontery'.\n"
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn no_config_found_enables_venery() {
    let output = Workspace::new().run(b"There was a group of alligators.");
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert_eq!(
        output.stdout,
        b"<stdin>:1:13: terms.venery: The venery term is 'congregation of alligators'.\n"
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn no_config_found_enables_garner_cliches() {
    let output = Workspace::new().run(b"A fate worse than death.");
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert_eq!(
        output.stdout,
        "<stdin>:1:1: cliches.misc.garner: 'A fate worse than death' is a cliché\n".as_bytes()
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn no_config_found_enables_corporate_speak() {
    let output = Workspace::new().run(b"synergy");
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert_eq!(
        output.stdout,
        b"<stdin>:1:1: industrial_language.corporate_speak: Minimize your use of corporate catchphrases like 'synergy'.\n"
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn no_config_found_enables_commercialese() {
    let output = Workspace::new().run(b"Please return same.");
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert_eq!(
        output.stdout,
        b"<stdin>:1:1: industrial_language.commercialese: 'Please return same' is commercialese.\n"
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn no_config_found_enables_spelling_consistency() {
    let output = Workspace::new().run(b"The adviser and advisor reviewed the report.");
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert_eq!(
        output.stdout,
        b"<stdin>:1:17: spelling.consistency: Inconsistent spelling of 'adviser' (vs. 'advisor').\n"
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn no_config_found_keeps_clean_input_clean() {
    let output = Workspace::new().run(b"An ordinary sentence.");
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn no_config_found_keeps_default_disabled_curly_quotes_check_disabled() {
    let output = Workspace::new().run(b"The phrase \"word\" is quoted.");
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}
