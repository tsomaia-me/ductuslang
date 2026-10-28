use crate::semantics::Internal;

#[derive(Clone, Copy)]
struct Mark {
    index: Option<usize>,
    low: usize,
    on_stack: bool,
}

pub(crate) fn strongly_connected(edges: &[Vec<usize>]) -> Result<Vec<Vec<usize>>, Internal> {
    let mut marks = vec![
        Mark {
            index: None,
            low: 0,
            on_stack: false,
        };
        edges.len()
    ];
    let mut stack = Vec::new();
    let mut components = Vec::new();
    let mut next = 0;
    for root in 0..edges.len() {
        if mark(&mut marks, root)?.index.is_some() {
            continue;
        }
        let mut work: Vec<(usize, usize)> = vec![(root, 0)];
        visit(root, &mut next, &mut marks, &mut stack)?;
        while let Some(&mut (node, ref mut edge)) = work.last_mut() {
            crate::semantics::counted_walk::CountedWalk::StronglyConnectedStronglyConnected.step();
            let targets = edges.get(node).ok_or(Internal::missed("graph node"))?;
            if let Some(&target) = targets.get(*edge) {
                *edge += 1;
                let reached = *mark(&mut marks, target)?;
                match reached.index {
                    None => {
                        visit(target, &mut next, &mut marks, &mut stack)?;
                        work.push((target, 0));
                    }
                    Some(index) if reached.on_stack => {
                        let current = mark(&mut marks, node)?;
                        current.low = current.low.min(index);
                    }
                    Some(_) => {}
                }
                continue;
            }
            work.pop();
            let finished = *mark(&mut marks, node)?;
            if let Some(&(parent, _)) = work.last() {
                let parent = mark(&mut marks, parent)?;
                parent.low = parent.low.min(finished.low);
            }
            if finished.index == Some(finished.low) {
                let mut component = Vec::new();
                loop {
                    let member = stack.pop().ok_or(Internal::missed("graph stack"))?;
                    mark(&mut marks, member)?.on_stack = false;
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
    Ok(components)
}

fn mark(marks: &mut [Mark], node: usize) -> Result<&mut Mark, Internal> {
    marks.get_mut(node).ok_or(Internal::missed("graph node"))
}

fn visit(
    node: usize,
    next: &mut usize,
    marks: &mut [Mark],
    stack: &mut Vec<usize>,
) -> Result<(), Internal> {
    *mark(marks, node)? = Mark {
        index: Some(*next),
        low: *next,
        on_stack: true,
    };
    *next += 1;
    stack.push(node);
    Ok(())
}
