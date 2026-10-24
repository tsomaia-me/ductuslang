const UNSEEN: usize = usize::MAX;

pub(crate) fn strongly_connected(edges: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let count = edges.len();
    let mut index = vec![UNSEEN; count];
    let mut low = vec![0; count];
    let mut on_stack = vec![false; count];
    let mut stack = Vec::new();
    let mut components = Vec::new();
    let mut next = 0;
    let seen = |index: &[usize], node: usize| index.get(node).is_some_and(|value| *value != UNSEEN);
    for root in 0..count {
        if seen(&index, root) {
            continue;
        }
        let mut work: Vec<(usize, usize)> = vec![(root, 0)];
        visit(
            root,
            &mut next,
            &mut index,
            &mut low,
            &mut on_stack,
            &mut stack,
        );
        while let Some(&mut (node, ref mut edge)) = work.last_mut() {
            let target = edges
                .get(node)
                .and_then(|targets| targets.get(*edge))
                .copied();
            if let Some(target) = target {
                *edge += 1;
                if !seen(&index, target) {
                    visit(
                        target,
                        &mut next,
                        &mut index,
                        &mut low,
                        &mut on_stack,
                        &mut stack,
                    );
                    work.push((target, 0));
                } else if on_stack.get(target).copied().unwrap_or(false) {
                    let reached = index.get(target).copied().unwrap_or(UNSEEN);
                    if let Some(slot) = low.get_mut(node) {
                        *slot = (*slot).min(reached);
                    }
                }
                continue;
            }
            work.pop();
            let node_low = low.get(node).copied().unwrap_or(0);
            if let Some(&(parent, _)) = work.last()
                && let Some(slot) = low.get_mut(parent)
            {
                *slot = (*slot).min(node_low);
            }
            if Some(node_low) == index.get(node).copied() {
                let mut component = Vec::new();
                while let Some(member) = stack.pop() {
                    if let Some(flag) = on_stack.get_mut(member) {
                        *flag = false;
                    }
                    component.push(member);
                    if member == node {
                        break;
                    }
                }
                component.reverse();
                components.push(component);
            }
        }
    }
    components
}

fn visit(
    node: usize,
    next: &mut usize,
    index: &mut [usize],
    low: &mut [usize],
    on_stack: &mut [bool],
    stack: &mut Vec<usize>,
) {
    if let Some(slot) = index.get_mut(node) {
        *slot = *next;
    }
    if let Some(slot) = low.get_mut(node) {
        *slot = *next;
    }
    *next += 1;
    stack.push(node);
    if let Some(flag) = on_stack.get_mut(node) {
        *flag = true;
    }
}
