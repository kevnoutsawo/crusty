//! Collection tree data structures.
//!
//! Collections organize requests into a hierarchical folder structure.
//! Folders can carry shared configuration (headers, auth, pre-request scripts)
//! that is inherited by all child requests.

use crate::request::RequestDefinition;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A collection is a named group of folders and requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    /// Unique identifier.
    pub id: Uuid,
    /// Collection name.
    pub name: String,
    /// Top-level items in this collection.
    pub items: Vec<CollectionItem>,
}

impl Collection {
    /// Create a new empty collection.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            items: Vec::new(),
        }
    }

    /// Add a request at the top level.
    pub fn add_request(&mut self, request: RequestDefinition) {
        self.items.push(CollectionItem::Request(request));
    }

    /// Add a folder at the top level.
    pub fn add_folder(&mut self, folder: Folder) {
        self.items.push(CollectionItem::Folder(folder));
    }

    /// Find a request by its ID anywhere in the tree.
    pub fn find_request(&self, id: &Uuid) -> Option<&RequestDefinition> {
        find_request_in_items(&self.items, id)
    }

    /// Find a request by its ID anywhere in the tree and return a mutable reference.
    pub fn find_request_mut(&mut self, id: &Uuid) -> Option<&mut RequestDefinition> {
        find_request_mut_in_items(&mut self.items, id)
    }

    /// Count all requests in this collection (recursively).
    pub fn request_count(&self) -> usize {
        count_requests_in_items(&self.items)
    }
}

/// An item in a collection: either a folder or a request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CollectionItem {
    /// A folder containing more items.
    Folder(Folder),
    /// A single request.
    Request(RequestDefinition),
}

/// A folder in the collection tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    /// Unique identifier.
    pub id: Uuid,
    /// Folder name.
    pub name: String,
    /// Child items.
    pub items: Vec<CollectionItem>,
}

impl Folder {
    /// Create a new empty folder.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            items: Vec::new(),
        }
    }

    /// Add a request to this folder.
    pub fn add_request(&mut self, request: RequestDefinition) {
        self.items.push(CollectionItem::Request(request));
    }

    /// Add a subfolder to this folder.
    pub fn add_folder(&mut self, folder: Folder) {
        self.items.push(CollectionItem::Folder(folder));
    }
}

fn find_request_in_items<'a>(items: &'a [CollectionItem], id: &Uuid) -> Option<&'a RequestDefinition> {
    for item in items {
        match item {
            CollectionItem::Request(req) if &req.id == id => return Some(req),
            CollectionItem::Folder(folder) => {
                if let Some(req) = find_request_in_items(&folder.items, id) {
                    return Some(req);
                }
            }
            _ => {}
        }
    }
    None
}

fn find_request_mut_in_items<'a>(
    items: &'a mut [CollectionItem],
    id: &Uuid,
) -> Option<&'a mut RequestDefinition> {
    for item in items {
        match item {
            CollectionItem::Request(req) if &req.id == id => return Some(req),
            CollectionItem::Folder(folder) => {
                if let Some(req) = find_request_mut_in_items(&mut folder.items, id) {
                    return Some(req);
                }
            }
            _ => {}
        }
    }
    None
}

fn count_requests_in_items(items: &[CollectionItem]) -> usize {
    items.iter().fold(0, |count, item| match item {
        CollectionItem::Request(_) => count + 1,
        CollectionItem::Folder(folder) => count + count_requests_in_items(&folder.items),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::RequestDefinition;

    #[test]
    fn test_collection_add_and_count() {
        let mut col = Collection::new("My API");
        col.add_request(RequestDefinition::new("Get Users", "https://api.example.com/users"));
        col.add_request(RequestDefinition::new("Get Posts", "https://api.example.com/posts"));

        assert_eq!(col.request_count(), 2);
    }

    #[test]
    fn test_nested_folder_count() {
        let mut col = Collection::new("My API");

        let mut folder = Folder::new("Auth");
        folder.add_request(RequestDefinition::new("Login", "https://api.example.com/login"));
        folder.add_request(RequestDefinition::new("Logout", "https://api.example.com/logout"));

        let mut subfolder = Folder::new("OAuth");
        subfolder.add_request(RequestDefinition::new("Token", "https://api.example.com/token"));
        folder.add_folder(subfolder);

        col.add_folder(folder);
        col.add_request(RequestDefinition::new("Health", "https://api.example.com/health"));

        assert_eq!(col.request_count(), 4);
    }

    #[test]
    fn test_find_request_by_id() {
        let mut col = Collection::new("Test");
        let req = RequestDefinition::new("Target", "https://example.com");
        let id = req.id;
        col.add_request(req);

        assert!(col.find_request(&id).is_some());
        assert_eq!(col.find_request(&id).unwrap().name, "Target");
    }

    #[test]
    fn test_find_request_in_nested_folder() {
        let mut col = Collection::new("Test");
        let mut folder = Folder::new("Nested");
        let req = RequestDefinition::new("Deep", "https://example.com");
        let id = req.id;
        folder.add_request(req);
        col.add_folder(folder);

        assert!(col.find_request(&id).is_some());
    }
}
