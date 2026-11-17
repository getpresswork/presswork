use hayagriva::types::{Date, EntryType, FormatString, Person, Publisher};
use hayagriva::{Entry, Library};
use presswork_api::Work;

#[derive(Debug, thiserror::Error)]
#[error("the bibliography cannot be written: {0}")]
pub struct BibliographyError(String);

pub fn bibliography(works: &[Work]) -> Result<String, BibliographyError> {
    let mut library = Library::new();
    for work in works {
        let mut entry = Entry::new(&work.key, EntryType::Book);
        entry.set_title(FormatString::with_value(work.title.as_str()));
        let authors: Vec<Person> = work
            .creators
            .iter()
            .filter_map(|name| name.parse().ok())
            .collect();
        if !authors.is_empty() {
            entry.set_authors(authors);
        }
        if let Some(edition) = work.standard_edition() {
            if let Some(year) = edition.year {
                entry.set_date(Date::from_year(year));
            }
            if !edition.publisher.is_empty() {
                let name = FormatString::with_value(edition.publisher.as_str());
                entry.set_publisher(Publisher::new(Some(name), None));
            }
        }
        library.push(&entry);
    }
    hayagriva::io::to_yaml_str(&library).map_err(|error| BibliographyError(error.to_string()))
}

#[cfg(test)]
mod tests {
    use hayagriva::io::from_yaml_str;
    use presswork_api::Edition;
    use uuid::Uuid;

    use super::*;

    fn edition(year: Option<i32>, publisher: &str, standard: bool) -> Edition {
        Edition {
            id: Uuid::nil(),
            year,
            publisher: publisher.to_owned(),
            language: String::new(),
            translators: Vec::new(),
            standard,
            file: None,
        }
    }

    fn work(key: &str, title: &str, creators: &[&str], year: Option<i32>) -> Work {
        Work {
            id: Uuid::nil(),
            organization_id: Uuid::nil(),
            key: key.to_owned(),
            title: title.to_owned(),
            creators: creators.iter().map(|name| (*name).to_owned()).collect(),
            editions: vec![edition(year, "", true)],
        }
    }

    #[test]
    fn the_standard_edition_gives_the_year_and_the_publisher() {
        let mut war = work("war-and-peace", "War and Peace", &["Tolstoy, Leo"], None);
        war.editions = vec![
            edition(Some(1869), "Russky Vestnik", false),
            edition(Some(1922), "Oxford University Press", true),
        ];

        let library = from_yaml_str(&bibliography(&[war]).unwrap()).unwrap();

        let entry = library.get("war-and-peace").unwrap();
        assert_eq!(entry.date().unwrap().year, 1922);
        assert_eq!(
            entry.publisher().unwrap().name().unwrap().to_string(),
            "Oxford University Press"
        );
    }

    #[test]
    fn each_work_becomes_an_entry_with_its_key() {
        let yaml = bibliography(&[
            work(
                "field-notes",
                "Field Notes: A Handbook",
                &["Gutenberg, Johannes", "Plato"],
                Some(1455),
            ),
            work("atlas", "Atlas of Presses", &[], None),
        ])
        .unwrap();

        let library = from_yaml_str(&yaml).unwrap();
        let notes = library.get("field-notes").unwrap();
        assert_eq!(
            notes.title().unwrap().to_string(),
            "Field Notes: A Handbook"
        );
        let names: Vec<(&str, Option<&str>)> = notes
            .authors()
            .unwrap()
            .iter()
            .map(|person| (person.name.as_str(), person.given_name.as_deref()))
            .collect();
        assert_eq!(names, [("Gutenberg", Some("Johannes")), ("Plato", None)]);
        assert_eq!(notes.date().unwrap().year, 1455);
        let atlas = library.get("atlas").unwrap();
        assert_eq!(atlas.title().unwrap().to_string(), "Atlas of Presses");
        assert!(atlas.authors().is_none());
        assert!(atlas.date().is_none());
    }
}
