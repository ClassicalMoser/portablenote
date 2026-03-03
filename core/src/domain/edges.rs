use uuid::Uuid;

use super::types::Edge;

pub fn create(id: Uuid, source: Uuid, target: Uuid) -> Edge {
    Edge { id, source, target }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_sets_fields() {
        let id = Uuid::parse_str("00000000-0000-4000-a000-0000000000e1").unwrap();
        let src = Uuid::parse_str("00000000-0000-4000-a000-000000000001").unwrap();
        let tgt = Uuid::parse_str("00000000-0000-4000-a000-000000000002").unwrap();
        let edge = create(id, src, tgt);
        assert_eq!(edge.id, id);
        assert_eq!(edge.source, src);
        assert_eq!(edge.target, tgt);
    }
}
