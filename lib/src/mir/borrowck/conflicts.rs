use crate::mir::{Place, Projection};

pub fn places_conflict(a: &Place, b: &Place) -> bool {
    if a.local != b.local {
        return false;
    }

    for (lhs, rhs) in a.projection.iter().zip(&b.projection) {
        if lhs != rhs {
            return match (lhs, rhs) {
                (Projection::Field { index: x, .. }, Projection::Field { index: y, .. })
                    if x != y =>
                {
                    false
                }
                _ => true,
            };
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::places_conflict;
    use crate::mir::{Local, Place, Projection};

    #[test]
    fn test_places_conflict_same_local_same_projection() {
        assert!(places_conflict(
            &Place {
                local: Local(1),
                projection: vec![],
            },
            &Place {
                local: Local(1),
                projection: vec![],
            },
        ));
    }

    #[test]
    fn test_places_conflict_disjoint_fields_do_not_overlap() {
        assert!(!places_conflict(
            &Place {
                local: Local(1),
                projection: vec![Projection::Field {
                    index: 0,
                    identity: None,
                }],
            },
            &Place {
                local: Local(1),
                projection: vec![Projection::Field {
                    index: 1,
                    identity: None,
                }],
            },
        ));
    }

    #[test]
    fn test_places_conflict_parent_and_field_overlap() {
        assert!(places_conflict(
            &Place {
                local: Local(1),
                projection: vec![],
            },
            &Place {
                local: Local(1),
                projection: vec![Projection::Field {
                    index: 0,
                    identity: None,
                }],
            },
        ));
    }
}
