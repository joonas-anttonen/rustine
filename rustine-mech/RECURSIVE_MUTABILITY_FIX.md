# Recursive Mutability Fix with Rc<RefCell<>> and std::mem::take()

## The Problem

The original DOM implementation had TWO recursive mutability issues:

### Issue 1: Node Children Mutation

```rust
// BEFORE - This doesn't compile!
fn compute_node_layout(
    &mut self,
    node: &mut UiNode,
    parent_x: f32,
    parent_y: f32,
    available_width: f32,
    available_height: f32,
) {
    // ... compute layout ...
    
    // Store in HashMap (needs &mut self)
    self.computed_layouts.insert(id.to_string(), computed_layout);
    
    // Recurse into children
    if let UiNode::Panel { children, .. } = node {
        for child in children.iter_mut() {  // Needs &mut node
            // ERROR: Cannot borrow self as mutable again while
            // also holding mutable reference to node!
            self.compute_node_layout(child, ...);
        }
    }
}
```

**Why This Failed:** Rust's borrow checker prevents having:
1. A mutable reference to `self` (to insert into HashMap)
2. A mutable reference to `node` (to iterate children mutably)
3. Another mutable reference to `self` (in the recursive call)

### Issue 2: Self HashMap Access During Recursion

Even after fixing Issue 1 with `Rc<RefCell<>>`, there was still a problem:

```rust
// STILL DOESN'T COMPILE!
fn compute_node_layout_recursive(&mut self, node: &UiNode, ...) {
    // Recursive call (needs &mut self)
    self.compute_node_layout_recursive(&child_rc.borrow(), ...);
    
    // ERROR: Also needs &self to read from HashMap!
    if let Some(computed) = self.computed_layouts.get(&child_id) {
        current_y += computed.height;
    }
}
```

**Why This Failed:** The recursive call borrows `self` mutably, but we also need to access `self.computed_layouts` in the same scope.

## The Solution: Two-Part Fix

### Part 1: Rc<RefCell<>> for Children

```rust
use std::rc::Rc;
use std::cell::RefCell;

// Children are now wrapped in Rc<RefCell<>>
pub enum UiNode {
    Panel {
        children: Vec<Rc<RefCell<UiNode>>>,  // Changed!
        // ... other fields
    },
    // ... other variants
}
```

This solves Issue 1 by allowing interior mutability.

### Part 2: std::mem::take() for Self Borrows

```rust
// Take temporary ownership of the HashMap
let mut layouts = std::mem::take(&mut self.computed_layouts);

// Now self.computed_layouts is empty, we own the original
// self can be borrowed mutably for recursion
self.compute_node_layout_recursive(&child_rc.borrow(), ...);

// Access the values from the recursive call (now in self.computed_layouts)
if let Some(computed) = self.computed_layouts.get(&child_id) {
    current_y += computed.height;
}

// Restore ownership - merge old values back
for (k, v) in layouts {
    self.computed_layouts.entry(k).or_insert(v);
}
```

This solves Issue 2 by temporarily moving the HashMap out of `self`.

## Implementation Details

### Part 1: Rc<RefCell<>> Pattern

**Updated Structure:**

```rust
// AFTER - This compiles!
fn compute_node_layout_recursive(
    &mut self,
    node: &UiNode,  // Immutable now!
    parent_x: f32,
    parent_y: f32,
    available_width: f32,
    available_height: f32,
) {
    // ... compute layout ...
    
    if let UiNode::Panel { children, .. } = node {
        for child_rc in children {
            // Borrow mutably just for the layout update
            {
                let mut child = child_rc.borrow_mut();
                child.layout_mut().y = current_y;
            } // Mutable borrow dropped here!
            
            // Now borrow immutably for the recursive call
            self.compute_node_layout_recursive(&child_rc.borrow(), ...);
        }
    }
}
```

### Part 2: std::mem::take() Pattern

**How std::mem::take() Works:**

```rust
pub fn take<T: Default>(dest: &mut T) -> T {
    std::mem::replace(dest, Default::default())
}
```

It replaces the value with a default (empty HashMap) and returns the original.

**Usage in Layout Computation:**

```rust
// VERTICAL LAYOUT
let mut layouts = std::mem::take(&mut self.computed_layouts);
// self.computed_layouts is now empty (Default::default())
// layouts contains the original HashMap

for child_rc in children {
    // Modify child
    child_rc.borrow_mut().layout_mut().y = current_y;
    
    // Recursive call fills self.computed_layouts with new values
    self.compute_node_layout_recursive(&child_rc.borrow(), ...);
    
    // Access NEW values from recursion
    if let Some(computed) = self.computed_layouts.get(&child_id) {
        current_y += computed.height;
    }
}

// Merge old values back (preserve new values from recursion)
for (k, v) in layouts {
    self.computed_layouts.entry(k).or_insert(v);
}
```

**Why .entry().or_insert():**
- New values from recursion are already in `self.computed_layouts`
- Old values from `layouts` need to be merged back
- `.or_insert()` only inserts if key doesn't exist (preserves recursion results)

**Usage in Hover State:**

```rust
// Take ownership before recursion
let mut states = std::mem::take(&mut self.element_states);

// Recursive calls can now borrow self mutably
for child_rc in node.children() {
    self.update_node_hover_state(&child_rc.borrow());
}

// Restore ownership
for (k, v) in states {
    self.element_states.entry(k).or_insert(v);
}
```

## Complete Example

### Before (Doesn't Compile)

```rust
fn compute_node_layout_recursive(&mut self, node: &UiNode, ...) {
    self.computed_layouts.insert(id.to_string(), computed);
    
    if let UiNode::Panel { children, .. } = node {
        for child_rc in children {
            // ERROR: Multiple mutable borrows of self
            self.compute_node_layout_recursive(&child_rc.borrow(), ...);
            
            // ERROR: Cannot borrow self as immutable while also borrowed as mutable
            if let Some(computed) = self.computed_layouts.get(&child_id) {
                current_y += computed.height;
            }
        }
    }
}
```

### After (Compiles Successfully)

```rust
fn compute_node_layout_recursive(&mut self, node: &UiNode, ...) {
    self.computed_layouts.insert(id.to_string(), computed);
    
    if let UiNode::Panel { children, .. } = node {
        // Take temporary ownership
        let mut layouts = std::mem::take(&mut self.computed_layouts);
        
        for child_rc in children {
            // Modify child via RefCell
            child_rc.borrow_mut().layout_mut().y = current_y;
            
            // Recursive call (self is free to be borrowed)
            self.compute_node_layout_recursive(&child_rc.borrow(), ...);
            
            // Access new values from recursion
            if let Some(computed) = self.computed_layouts.get(&child_id) {
                current_y += computed.height;
            }
        }
        
        // Restore old values
        for (k, v) in layouts {
            self.computed_layouts.entry(k).or_insert(v);
        }
    }
}
```

## Runtime Behavior

### std::mem::take() Cost

**Performance:**
- O(1) operation - just moving a pointer
- No data copying
- No allocations
- Essentially free

**Memory:**
- Creates one empty HashMap temporarily
- Original HashMap is moved, not copied
- Total: one extra empty HashMap on the stack

### Borrow Checking

RefCell enforces borrowing rules at runtime:

```rust
let node = Rc::new(RefCell::new(UiNode::Panel { ... }));

// OK: Multiple immutable borrows
let borrow1 = node.borrow();
let borrow2 = node.borrow();

// OK: Single mutable borrow
let mut borrow = node.borrow_mut();
borrow.layout_mut().x = 10.0;

// PANIC: Can't have mutable + immutable borrow
let immut = node.borrow();
let mut mut_borrow = node.borrow_mut();  // Panics!
```

## Performance Characteristics

### Memory Overhead

**Per Node:**
- Rc: 2 words (16 bytes on 64-bit)
- RefCell: 1 word (8 bytes)
- Total: ~24 bytes overhead per node

**Per HashMap Operation:**
- std::mem::take: 0 bytes (just pointer swap)
- Empty HashMap: ~24 bytes (temporary)

### Runtime Cost

- **Rc clone**: O(1) - increment reference count
- **borrow()**: O(1) - check and increment borrow counter
- **borrow_mut()**: O(1) - check borrow state is zero
- **std::mem::take()**: O(1) - pointer swap
- **HashMap merge**: O(n) where n = old HashMap size

Total: Very fast, suitable for UI rendering.

## When to Use This Pattern

### std::mem::take() is Good For:

✅ Recursive methods that need `&mut self`
✅ Temporarily removing a field from a struct
✅ Avoiding borrow conflicts with self-referential operations
✅ Single-threaded code

### Rc<RefCell<>> is Good For:

✅ Tree structures with parent-child relationships
✅ Graphs where nodes reference each other
✅ Interior mutability needed
✅ Single-threaded code
✅ Moderate performance requirements

### Combined Pattern is Perfect For:

✅ **Recursive tree traversal with state accumulation**
✅ **DOM-like structures**
✅ **Scene graphs**
✅ **AST processing**

## Alternative Solutions Considered

### 1. Two-Phase Algorithm
```rust
// Phase 1: Collect all layouts (no mutation)
fn collect_layouts(&self, node: &UiNode) -> Vec<Layout>;

// Phase 2: Apply layouts (mutation)
fn apply_layouts(&mut self, node: &mut UiNode, layouts: &[Layout]);
```

**Pros:** No interior mutability needed
**Cons:** Requires extra allocation, more complex, harder to maintain

### 2. Pass HashMap as Parameter
```rust
fn compute_layout_with_map(
    &mut self,
    node: &UiNode,
    layouts: &mut HashMap<String, ComputedLayout>
)
```

**Pros:** Avoids self borrow
**Cons:** Awkward API, exposes internals, error-prone

### 3. Indices Instead of References
```rust
struct NodeId(usize);
children: Vec<NodeId>,
```

**Pros:** No reference counting
**Cons:** Can become invalid, more complex API, less safe

## Why We Chose This Pattern

- ✅ **Standard Rust patterns** - Both are idiomatic
- ✅ **Safe** - No unsafe code
- ✅ **Simple** - Easy to understand once explained
- ✅ **Proven** - Used throughout Rust ecosystem
- ✅ **Performance** - Negligible overhead for UI rendering
- ✅ **Composable** - Both patterns work together naturally

## Summary

The complete solution requires BOTH patterns:

1. **Rc<RefCell<>> for children** - Solves node mutation conflicts
2. **std::mem::take() for HashMaps** - Solves self borrow conflicts

This maintains safety, adds minimal overhead, and represents idiomatic Rust for this problem space.

## References

- [Rust Book: Rc<T> Reference Counting](https://doc.rust-lang.org/book/ch15-04-rc.html)
- [Rust Book: RefCell<T> Interior Mutability](https://doc.rust-lang.org/book/ch15-05-interior-mutability.html)
- [std::mem::take Documentation](https://doc.rust-lang.org/std/mem/fn.take.html)
- [Rust API Guidelines: Common ownership patterns](https://rust-lang.github.io/api-guidelines/)
