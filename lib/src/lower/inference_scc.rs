use std::collections::{BTreeSet, HashMap, HashSet};

use crate::ids::DefId;

pub(crate) fn components(
    nodes: &BTreeSet<DefId>,
    edges: &HashMap<DefId, BTreeSet<DefId>>,
) -> HashMap<DefId, DefId> {
    fn visit(
        node: DefId,
        edges: &HashMap<DefId, BTreeSet<DefId>>,
        visited: &mut HashSet<DefId>,
        order: &mut Vec<DefId>,
    ) {
        if !visited.insert(node) {
            return;
        }
        if let Some(neighbors) = edges.get(&node) {
            for neighbor in neighbors {
                visit(*neighbor, edges, visited, order);
            }
        }
        order.push(node);
    }

    fn visit_reverse(
        node: DefId,
        reverse: &HashMap<DefId, BTreeSet<DefId>>,
        assigned: &HashSet<DefId>,
        component: &mut BTreeSet<DefId>,
    ) {
        if assigned.contains(&node) || !component.insert(node) {
            return;
        }
        if let Some(neighbors) = reverse.get(&node) {
            for neighbor in neighbors {
                visit_reverse(*neighbor, reverse, assigned, component);
            }
        }
    }

    let mut order = Vec::with_capacity(nodes.len());
    let mut visited = HashSet::new();
    for node in nodes {
        visit(*node, edges, &mut visited, &mut order);
    }

    let mut reverse = HashMap::<DefId, BTreeSet<DefId>>::new();
    for node in nodes {
        reverse.entry(*node).or_default();
    }
    for (source, targets) in edges {
        for target in targets {
            reverse.entry(*target).or_default().insert(*source);
        }
    }

    let mut result = HashMap::new();
    let mut assigned = HashSet::new();
    for node in order.into_iter().rev() {
        if assigned.contains(&node) {
            continue;
        }
        let mut component = BTreeSet::new();
        visit_reverse(node, &reverse, &assigned, &mut component);
        assigned.extend(component.iter().copied());
        let representative = *component.first().expect("SCC traversal visits its root");
        for member in component {
            result.insert(member, representative);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{CrateId, LocalDefId};

    #[test]
    fn components_merge_recursive_edges_deterministically() {
        let id = |local| DefId::new(CrateId(0), LocalDefId(local));
        let nodes = [id(1), id(2), id(3)].into_iter().collect();
        let edges = HashMap::from([
            (id(1), BTreeSet::from([id(2)])),
            (id(2), BTreeSet::from([id(1), id(3)])),
            (id(3), BTreeSet::new()),
        ]);
        let result = components(&nodes, &edges);
        assert_eq!(result[&id(1)], id(1));
        assert_eq!(result[&id(2)], id(1));
        assert_eq!(result[&id(3)], id(3));
    }
}
