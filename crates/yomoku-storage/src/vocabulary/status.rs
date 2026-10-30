use super::{StorageError, VocabularyStatus};

pub(crate) fn decode_status(status: &str) -> Result<VocabularyStatus, StorageError> {
    match status {
        "new" => Ok(VocabularyStatus::New),
        "learning" => Ok(VocabularyStatus::Learning),
        "familiar" => Ok(VocabularyStatus::Familiar),
        "known" => Ok(VocabularyStatus::Known),
        "ignored" => Ok(VocabularyStatus::Ignored),
        _ => Err(StorageError::InvalidVocabulary),
    }
}
