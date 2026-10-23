//! Debug-APK measurement boundary, excluded from the production bridge.
//! JNI primitives only: no Java references or interface pointers are retained.

use std::{
    cell::RefCell,
    ffi::c_void,
    fs::File,
    os::fd::BorrowedFd,
    panic::{catch_unwind, AssertUnwindSafe},
    sync::atomic::{AtomicI64, Ordering},
};
use yomoku_analysis::Analyzer;
use yomoku_core::{analysis::SplitMode, document::ChapterText};

const INVALID_INPUT: i64 = -1;
const ALREADY_OPEN: i64 = -2;
const CLOSED_OR_WRONG_WORKER: i64 = -3;
const LOAD_FAILED: i64 = -4;
const VALIDATION_FAILED: i64 = -5;
const PANICKED: i64 = -99;

struct Session {
    generation: i64,
    analyzer: Analyzer,
}

thread_local! {
    // Sudachi never crosses a worker boundary. Thread exit drops any live mapping.
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
}
static NEXT_GENERATION: AtomicI64 = AtomicI64::new(1);

fn boundary(operation: impl FnOnce() -> Result<i64, i64>) -> i64 {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(result) => result.unwrap_or_else(|code| code),
        Err(_) => {
            // Do not reuse potentially inconsistent analyzer state after a panic.
            SESSION.with(|slot| *slot.borrow_mut() = None);
            PANICKED
        }
    }
}

/// # Safety
/// `fd` must remain open during this synchronous call. Its backing inode must
/// remain immutable until nativeClose or worker exit, including on failure.
/// Only the controlled private-file publisher/test fixtures may supply it.
#[no_mangle]
pub unsafe extern "system" fn Java_app_yomoku_SudachiAppProbe_nativeOpen(
    _env: *mut c_void,
    _object: *mut c_void,
    fd: i32,
) -> i64 {
    boundary(|| {
        if fd < 0 {
            return Err(INVALID_INPUT);
        }
        SESSION.with(|slot| {
            let mut slot = slot.borrow_mut();
            if slot.is_some() {
                return Err(ALREADY_OPEN);
            }
            // SAFETY: Kotlin holds its verified descriptor open across this call.
            // Duplicate that exact description, never reopen a pathname.
            let borrowed = unsafe { BorrowedFd::borrow_raw(fd) };
            let file = File::from(borrowed.try_clone_to_owned().map_err(|_| LOAD_FAILED)?);
            // SAFETY: the private publisher never rewrites published inodes;
            // tests clean their isolated directory only after worker shutdown.
            // This constructor also rehashes the actual mapping itself.
            let analyzer = unsafe { Analyzer::open_full_mapped(&file) }.map_err(|_| LOAD_FAILED)?;
            let generation = NEXT_GENERATION
                .try_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
                .map_err(|_| LOAD_FAILED)?;
            *slot = Some(Session {
                generation,
                analyzer,
            });
            Ok(generation)
        })
    })
}

// Fixed validated fixtures keep measurement-only input/output bounded. No book
// import, persistence or application state is reachable through this boundary.
#[no_mangle]
pub extern "system" fn Java_app_yomoku_SudachiAppProbe_nativeValidate(
    _env: *mut c_void,
    _object: *mut c_void,
    generation: i64,
    fixture: i32,
) -> i64 {
    boundary(|| {
        SESSION.with(|slot| {
            let slot = slot.borrow();
            let session = slot
                .as_ref()
                .filter(|session| session.generation == generation)
                .ok_or(CLOSED_OR_WRONG_WORKER)?;
            validate(&session.analyzer, fixture)
        })
    })
}

fn validate(analyzer: &Analyzer, fixture: i32) -> Result<i64, i64> {
    let (text, mode, expected): (String, SplitMode, Option<&[&str]>) = match fixture {
        0 => (
            "国家公務員".into(),
            SplitMode::A,
            Some(&["国家", "公務", "員"]),
        ),
        1 => ("国家公務員".into(), SplitMode::B, Some(&["国家", "公務員"])),
        2 => ("国家公務員".into(), SplitMode::C, Some(&["国家公務員"])),
        3 => (
            "English 123 📖 日本語。𰻞 e\u{301}と𠮷\0猫 ｶﾞｯﾂﾎﾟｰｽﾞ".into(),
            SplitMode::B,
            None,
        ),
        4 => (
            "猫と犬。東京都と夏目漱石。食べる。".repeat(30),
            SplitMode::B,
            None,
        ),
        5 => ("猫と犬".repeat(8000), SplitMode::B, None),
        _ => return Err(INVALID_INPUT),
    };
    let chapter = ChapterText {
        source_id: 17.try_into().map_err(|_| VALIDATION_FAILED)?,
        chapter_id: 29.try_into().map_err(|_| VALIDATION_FAILED)?,
        text,
    };
    let output = analyzer
        .analyze_mode(&chapter, mode)
        .map_err(|_| VALIDATION_FAILED)?;
    output.validate(&chapter).map_err(|_| VALIDATION_FAILED)?;
    let surfaces: Vec<_> = output
        .tokens
        .iter()
        .map(|token| token.surface.as_str())
        .collect();
    if let Some(expected) = expected {
        if surfaces != expected {
            return Err(VALIDATION_FAILED);
        }
    }
    if surfaces.concat() != chapter.text.replace(' ', "") {
        return Err(VALIDATION_FAILED);
    }
    if fixture == 3 {
        let rare = output
            .tokens
            .iter()
            .find(|token| token.surface == "𰻞")
            .ok_or(VALIDATION_FAILED)?;
        if !rare.is_japanese || rare.dictionary_form.is_some() || rare.reading.is_some() {
            return Err(VALIDATION_FAILED);
        }
        for surface in ["English", "123", "📖", "。"] {
            if !output
                .tokens
                .iter()
                .any(|token| token.surface == surface && !token.is_japanese)
            {
                return Err(VALIDATION_FAILED);
            }
        }
    }
    Ok(output.tokens.len() as i64)
}

#[no_mangle]
pub extern "system" fn Java_app_yomoku_SudachiAppProbe_nativeClose(
    _env: *mut c_void,
    _object: *mut c_void,
    generation: i64,
) -> i64 {
    boundary(|| {
        SESSION.with(|slot| {
            let mut slot = slot.borrow_mut();
            if !slot
                .as_ref()
                .is_some_and(|session| session.generation == generation)
            {
                return Err(CLOSED_OR_WRONG_WORKER);
            }
            *slot = None;
            Ok(0)
        })
    })
}
