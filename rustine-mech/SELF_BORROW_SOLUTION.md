# Self Borrow Conflict Resolution - Complete Guide

## Problem Statement

> "the problem remains with self borrows in the recursive call. need to manage the state hash maps carefully as well (possibly take temporary ownership for the call and then take ownership back after returning from call)"

## Solution Overview

The recursive DOM layout computation had overlapping mutable borrow conflicts that required a two-part solution:

1. **Rc<RefCell<>> for node children** - Enables interior mutability
2. **std::mem::take() for state HashMaps** - Enables temporary ownership transfer

## The Borrow Conflict

### Code That Doesn't Compile

```rust
fn compute_node_layout_recursive(&mut self, node: &UiNode, ...) {
    // Insert into HashMap
    self.computed_layouts.insert(id.to_string(), layout);
    
    // Loop over children
    for child_rc in children {
        // Recursive call - borrows &mut self
        self.compute_node_layout_recursive(&child_rc.borrow(), ...);
        
        // ERROR: Can't access self.computed_layouts here
        // because self is still borrowed mutably above!
        if let Some(computed) = self.computed_layouts.get(&child_id) {
            current_y += computed.height;
        }
    }
}
```

**The Issue:**
- Line 7: Recursive call needs `&mut self`
- Line 11: Reading from `self.computed_layouts` needs `&self`
- These borrows overlap in the same scope!

### Why This Happens

In Rust, you cannot:
1. Borrow `self` mutably for a recursive call
2. Then borrow `self` (even immutably) to access fields
3. While the first borrow is still in scope

The loop creates a scope where the recursive call's mutable borrow of `self` extends through the entire iteration, preventing any subsequent access to `self`.

## The Solution: std::mem::take()

### What It Does

```rust
pub fn take<T: Default>(dest: &mut T) -> T {
    std::mem::replace(dest, Default::default())
}
```

`std::mem::take()`:
1. Replaces the HashMap with an empty default
2. Returns the original HashMap (now owned by the caller)
3. Frees up `self` for mutable borrowing

### How We Use It

```rust
fn compute_node_layout_recursive(&mut self, node: &UiNode, ...) {
    // Insert current node's layout
    self.computed_layouts.insert(id.to_string(), layout);
    
    // === KEY STEP: Take ownership ===
    let mut layouts = std::mem::take(&mut self.computed_layouts);
    // Now: layouts contains the original HashMap
    //      self.computed_layouts is empty (Default::default())
    
    // Loop over children
    for child_rc in children {
        // Modify child layout
        child_rc.borrow_mut().layout_mut().y = current_y;
        
        // Recursive call - self is free to be borrowed!
        // This will populate self.computed_layouts with child layouts
        self.compute_node_layout_recursive(&child_rc.borrow(), ...);
        
        // Access the NEW layout from the recursive call
        if let Some(computed) = self.computed_layouts.get(&child_id) {
            current_y += computed.height;
        }
    }
    
    // === KEY STEP: Restore ownership ===
    // Merge old layouts back (but preserve new values from recursion)
    for (k, v) in layouts {
        self.computed_layouts.entry(k).or_insert(v);
    }
}
```

### Flow Diagram

```
Before loop:
  self.computed_layouts = {parent: {...}}
  layouts = (empty)

After std::mem::take():
  self.computed_layouts = {} (empty)
  layouts = {parent: {...}}

During recursion:
  self.computed_layouts = {child1: {...}, child2: {...}}
  layouts = {parent: {...}}

After merging:
  self.computed_layouts = {parent: {...}, child1: {...}, child2: {...}}
  layouts = (dropped)
```

## Why .entry().or_insert()?

We merge with `.entry().or_insert()` instead of just moving the HashMap back:

```rust
// DON'T DO THIS - loses recursive results!
self.computed_layouts = layouts;

// DO THIS - preserves recursive results!
for (k, v) in layouts {
    self.computed_layouts.entry(k).or_insert(v);
}
```

**Reason:**
- `self.computed_layouts` contains NEW values from recursion (child layouts)
- `layouts` contains OLD values from before recursion (parent layout)
- We want BOTH sets of values in the final HashMap
- `.or_insert()` only inserts if the key doesn't exist (preserves child layouts)

## Pattern Applied to Multiple Methods

### 1. Vertical Layout

```rust
LayoutMode::Vertical => {
    let mut layouts = std::mem::take(&mut self.computed_layouts);
    
    let mut current_y = 0.0;
    for child_rc in children {
        child_rc.borrow_mut().layout_mut().y = current_y;
        self.compute_node_layout_recursive(&child_rc.borrow(), ...);
        
        if let Some(computed) = self.computed_layouts.get(&child_id) {
            current_y += computed.height;
        }
    }
    
    for (k, v) in layouts {
        self.computed_layouts.entry(k).or_insert(v);
    }
}
```

### 2. Horizontal Layout

```rust
LayoutMode::Horizontal => {
    let mut layouts = std::mem::take(&mut self.computed_layouts);
    
    let mut current_x = 0.0;
    for child_rc in children {
        child_rc.borrow_mut().layout_mut().x = current_x;
        self.compute_node_layout_recursive(&child_rc.borrow(), ...);
        
        if let Some(computed) = self.computed_layouts.get(&child_id) {
            current_x += computed.width;
        }
    }
    
    for (k, v) in layouts {
        self.computed_layouts.entry(k).or_insert(v);
    }
}
```

### 3. Hover State Updates

```rust
fn update_node_hover_state(&mut self, node: &UiNode) {
    // Compute and insert current node's state
    if let Some(id) = node.id() {
        self.element_states.insert(id.to_string(), state);
    }
    
    // Take ownership before recursion
    let mut states = std::mem::take(&mut self.element_states);
    
    // Recurse into children
    for child_rc in node.children() {
        self.update_node_hover_state(&child_rc.borrow());
    }
    
    // Restore ownership
    for (k, v) in states {
        self.element_states.entry(k).or_insert(v);
    }
}
```

## Performance Impact

### std::mem::take() Cost

| Operation | Cost | Description |
|-----------|------|-------------|
| `std::mem::take()` | O(1) | Just swaps pointers |
| Empty HashMap creation | O(1) | Default::default() |
| HashMap iteration | O(n) | Where n = old entries |
| `.entry().or_insert()` | O(1) amortized | Per entry |

**Total:** O(n) where n is the number of entries in the old HashMap, but n is typically small (number of sibling nodes at each level).

### Memory Usage

- One extra empty HashMap on the stack per recursion level
- Size: ~24 bytes (typical HashMap overhead)
- Temporary: Dropped after merging

**Impact:** Negligible for typical UI trees (depth < 20).

## Alternative Approaches Considered

### 1. Pass HashMap as Parameter

```rust
fn compute_layout_with_map(
    &mut self,
    node: &UiNode,
    layouts: &mut HashMap<String, ComputedLayout>
)
```

**Rejected because:**
- Awkward API
- Exposes internal implementation
- Error-prone (caller must manage HashMap)

### 2. Collect-Then-Apply

```rust
fn collect_layouts(&self) -> HashMap<String, ComputedLayout> {
    // Phase 1: Collect all layouts
}

fn apply_layouts(&mut self, layouts: HashMap<String, ComputedLayout>) {
    // Phase 2: Apply to self
}
```

**Rejected because:**
- Requires two passes over the tree
- Extra allocation
- More complex
- Doesn't fully solve the recursion problem

### 3. RefCell for HashMaps

```rust
struct UiDom {
    computed_layouts: RefCell<HashMap<String, ComputedLayout>>,
}
```

**Rejected because:**
- Adds runtime overhead to every HashMap access
- Can panic at runtime if borrow rules violated
- Less clear ownership semantics
- std::mem::take is more idiomatic

## When to Use This Pattern

### Indicators You Need std::mem::take()

✅ Recursive method with `&mut self`
✅ Need to access `self.field` after recursive call
✅ Can't restructure to avoid the conflict
✅ The field can be temporarily empty (has Default impl)

### Prerequisites

The field must implement `Default`:
- ✅ HashMap - Yes (empty HashMap)
- ✅ Vec - Yes (empty Vec)
- ✅ Option - Yes (None)
- ✅ String - Yes (empty String)
- ❌ Custom types - Must impl Default

## Best Practices

### DO:

✅ Use `std::mem::take()` for recursive self borrows
✅ Merge values back with `.entry().or_insert()`
✅ Document why you're doing it (link to this guide!)
✅ Keep the scope of temporary ownership small

### DON'T:

❌ Forget to merge values back
❌ Use `std::mem::replace()` with a new HashMap (loses data)
❌ Hold the temporary value longer than necessary
❌ Use this pattern when simpler solutions exist

## Testing Strategy

### Verify Correctness

```rust
#[test]
fn test_layout_computation() {
    let mut dom = UiDom::new();
    // ... build DOM with nested children ...
    
    dom.compute_layout(800.0, 600.0);
    
    // All nodes should have computed layouts
    assert!(dom.get_computed_layout("parent").is_some());
    assert!(dom.get_computed_layout("child1").is_some());
    assert!(dom.get_computed_layout("child2").is_some());
}
```

### Verify No Panics

```rust
#[test]
fn test_deep_nesting() {
    let mut dom = UiDom::new();
    // ... build deeply nested structure ...
    
    // Should not panic from RefCell borrow violations
    dom.compute_layout(800.0, 600.0);
    dom.update_mouse(100.0, 100.0, false, false);
}
```

## Summary

The complete solution to recursive self borrow conflicts:

1. **Identify the conflict** - Recursive call needs `&mut self`, but you also need to access `self.field`
2. **Use std::mem::take()** - Temporarily move the field out of `self`
3. **Perform recursion** - Now `self` can be borrowed freely
4. **Merge values back** - Use `.entry().or_insert()` to preserve both old and new values

This pattern, combined with `Rc<RefCell<>>` for children, completely eliminates borrow conflicts in recursive tree traversal.

## References

- [std::mem::take Documentation](https://doc.rust-lang.org/std/mem/fn.take.html)
- [std::mem::replace Documentation](https://doc.rust-lang.org/std/mem/fn.replace.html)
- [HashMap::entry Documentation](https://doc.rust-lang.org/std/collections/struct.HashMap.html#method.entry)
- [Rust Nomicon: Working with Uninitialized Memory](https://doc.rust-lang.org/nomicon/uninitialized.html)
