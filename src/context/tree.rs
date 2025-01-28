use crate::context::{get_component, query_entities, Context, EntityId, PARENT};

#[derive(Default, Debug, Clone, PartialEq)]
pub struct Name(pub String);

#[derive(Default, Debug, Copy, Clone, PartialEq)]
pub struct Parent(pub crate::context::EntityId);

// Query for the child entities of an entity
pub fn query_children(context: &Context, target_entity: EntityId) -> Vec<EntityId> {
    let mut child_entities = Vec::new();
    query_entities(context, PARENT)
        .into_iter()
        .for_each(|entity| {
            let Some(Parent(parent_entity)) = get_component(context, entity, PARENT) else {
                return;
            };
            if *parent_entity != target_entity {
                return;
            }
            child_entities.push(entity);
        });
    child_entities
}

/// Query for all the descendent entities of a target entity
pub fn query_descendents(context: &Context, target_entity: EntityId) -> Vec<EntityId> {
    let mut descendents = Vec::new();
    let mut stack = vec![target_entity];
    while let Some(entity) = stack.pop() {
        descendents.push(entity);
        query_children(context, entity)
            .into_iter()
            .for_each(|child| {
                stack.push(child);
            });
    }
    descendents
}

pub fn is_descendant_of(
    context: &crate::context::Context,
    entity: EntityId,
    ancestor: EntityId,
) -> bool {
    if entity == ancestor {
        return true;
    }

    let mut current = entity;
    while let Some(Parent(parent)) = get_component::<Parent>(context, current, PARENT) {
        if *parent == ancestor {
            return true;
        }
        current = *parent;
    }
    false
}
