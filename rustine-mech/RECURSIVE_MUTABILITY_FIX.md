# Recursive Mutability Fix with Rc<RefCell<>>

## The Problem

The original DOM implementation had a recursive mutability issue in the `compute_node_layout` method:

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

### Why This Failed

Rust's borrow checker prevents having:
1. A mutable reference to `self` (to insert into HashMap)
2. A mutable reference to `node` (to iterate children mutably)
3. Another mutable reference to `self` (in the recursive call)

All at the same time! The recursive structure creates overlapping mutable borrows.

## The Solution: Rc<RefCell<>>

The standard Rust pattern for tree structures with interior mutability:

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

### How It Works

**Rc<T>** - Reference Counting
- Allows multiple owners of the same data
- Non-atomic (single-threaded only)
- Automatically cleans up when last reference drops

**RefCell<T>** - Interior Mutability
- Allows mutation through shared reference
- Enforces borrowing rules at runtime (panics on violation)
- Perfect for tree structures

**Combined: Rc<RefCell<T>>**
- Multiple parts can reference the same node
- Can mutate through shared references
- Safe for single-threaded code

## Implementation Details

### Updated Structure

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
    
    // Store in HashMap (still needs &mut self, but that's OK)
    self.computed_layouts.insert(id.to_string(), computed_layout);
    
    if let UiNode::Panel { children, .. } = node {
        for child_rc in children {
            // Borrow mutably just for the layout update
            {
                let mut child = child_rc.borrow_mut();
                child.layout_mut().y = current_y;
            } // Mutable borrow dropped here!
            
            // Now borrow immutably for the recursive call
            self.compute_node_layout_recursive(
                &child_rc.borrow(),  // Immutable borrow
                x, y, width, height
            );
        }
    }
}
```

### Key Changes

1. **Children Type Changed**
   ```rust
   // Before
   children: Vec<UiNode>
   
   // After
   children: Vec<Rc<RefCell<UiNode>>>
   ```

2. **Access Pattern Changed**
   ```rust
   // Before (mutable)
   for child in children.iter_mut() {
       child.layout_mut().y = current_y;
       self.recurse(child);
   }
   
   // After (RefCell)
   for child_rc in children {
       child_rc.borrow_mut().layout_mut().y = current_y;
       self.recurse(&child_rc.borrow());
   }
   ```

3. **Construction Changed**
   ```rust
   // Before
   children: vec![
       UiNode::Text { ... },
       UiNode::Button { ... },
   ]
   
   // After
   children: vec![
       Rc::new(RefCell::new(UiNode::Text { ... })),
       Rc::new(RefCell::new(UiNode::Button { ... })),
   ]
   ```

## Runtime Behavior

### Borrow Checking

RefCell enforces borrowing rules at runtime:

```rust
let node = Rc::new(RefCell::new(UiNode::Panel { ... }));

// OK: Multiple immutable borrows
let borrow1 = node.borrow();
let borrow2 = node.borrow();
// Both can read simultaneously

// OK: Single mutable borrow
let mut borrow = node.borrow_mut();
borrow.layout_mut().x = 10.0;

// PANIC: Can't have mutable + immutable borrow
let immut = node.borrow();
let mut mut_borrow = node.borrow_mut();  // Panics!
```

### Our Usage Pattern (Safe)

```rust
// 1. Borrow mutably, modify, drop borrow
{
    let mut child = child_rc.borrow_mut();
    child.layout_mut().y = current_y;
}  // Mutable borrow dropped

// 2. Borrow immutably for read-only access
self.compute_layout(&child_rc.borrow());

// No conflict because borrows don't overlap!
```

## Performance Characteristics

### Memory Overhead

- **Rc**: 2 words (8 bytes on 64-bit) per node
  - Strong count
  - Weak count
- **RefCell**: 1 word (4 bytes) per node
  - Borrow state (count of borrows)

Total: ~12 bytes overhead per node

### Runtime Cost

- **Rc clone**: Increment reference count (very cheap)
- **borrow()**: Check borrow state, increment counter (cheap)
- **borrow_mut()**: Check borrow state is zero, set flag (cheap)
- **drop**: Decrement counter, possibly deallocate (cheap)

All operations are O(1) and very fast.

### Comparison to Alternatives

| Approach | Pros | Cons |
|----------|------|------|
| **Raw pointers** | Zero overhead | Unsafe, easy to misuse |
| **Box<T>** | Simple ownership | Can't share nodes |
| **Rc<RefCell<T>>** | Safe sharing, interior mutability | Small runtime overhead |
| **Arc<Mutex<T>>** | Thread-safe | Heavier (needed for threading) |

For single-threaded UI: **Rc<RefCell<T>> is the sweet spot**

## When to Use Rc<RefCell<>>

✅ **Good for:**
- Tree structures with parent-child relationships
- Graphs where nodes reference each other
- Interior mutability needed
- Single-threaded code
- Moderate performance requirements

❌ **Not ideal for:**
- Multi-threaded code (use Arc<Mutex<>> instead)
- High-performance hot paths (use indices or arena allocation)
- Simple linear structures (use Vec<T>)

## Alternative Solutions Considered

### 1. Arena Allocation with Indices
```rust
struct UiArena {
    nodes: Vec<UiNode>,
}

struct NodeId(usize);

// Access by index instead of reference
let child_id = NodeId(5);
let child = &arena.nodes[child_id.0];
```

**Pros:** No reference counting overhead
**Cons:** More complex API, indices can become invalid

### 2. Unsafe Raw Pointers
```rust
children: Vec<*mut UiNode>
```

**Pros:** Zero overhead
**Cons:** Unsafe, error-prone, defeats Rust's safety guarantees

### 3. Two-Phase Algorithm
```rust
// Phase 1: Collect all layouts (no mutation)
fn collect_layouts(&self, node: &UiNode) -> Vec<Layout>;

// Phase 2: Apply layouts (mutation)
fn apply_layouts(&mut self, node: &mut UiNode, layouts: &[Layout]);
```

**Pros:** Avoids interior mutability
**Cons:** Requires extra allocation, more complex, still has issues with recursion

### Why We Chose Rc<RefCell<>>

- ✅ **Standard Rust pattern** for trees
- ✅ **Safe** - No unsafe code
- ✅ **Simple** - Easy to understand and maintain
- ✅ **Proven** - Used throughout Rust ecosystem (e.g., rustc itself)
- ✅ **Performance** - Negligible overhead for UI rendering

## Summary

The recursive mutability issue was solved by:

1. Wrapping children in `Rc<RefCell<UiNode>>`
2. Borrowing mutably only when needed, dropping before recursion
3. Using immutable borrows for recursive traversal
4. Following Rust's standard pattern for tree structures

This maintains safety, adds minimal overhead, and is the idiomatic Rust solution for this problem.

## References

- [Rust Book: Rc<T> Reference Counting](https://doc.rust-lang.org/book/ch15-04-rc.html)
- [Rust Book: RefCell<T> Interior Mutability](https://doc.rust-lang.org/book/ch15-05-interior-mutability.html)
- [Rust API Guidelines: Common ownership patterns](https://rust-lang.github.io/api-guidelines/)
