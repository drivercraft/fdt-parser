//! Iterator for traversing all nodes in a Flattened Device Tree.
//!
//! This module provides `FdtIter`, which walks through the structure block
//! of an FDT and yields each node in a depth-first traversal order.

use log::error;

use crate::{
    Fdt, FdtError, Node, NodeContext, Token,
    data::{Bytes, Reader},
    node::{OneNodeIter, OneNodeState},
};

/// Iterator over all nodes in a Flattened Device Tree.
///
/// This iterator performs a depth-first traversal of the device tree,
/// yielding each node as it's encountered. It maintains a context stack
/// to track the `#address-cells` and `#size-cells` values inherited from
/// parent nodes.
pub struct FdtIter<'a> {
    fdt: Fdt<'a>,
    reader: Reader<'a>,
    strings: Bytes<'a>,
    /// The node iterator currently being processed
    node_iter: Option<OneNodeIter<'a>>,
    /// Whether iteration has terminated (due to error or end)
    finished: bool,
    /// Current depth level in the tree
    level: usize,
    /// Context stack, with the top being the current context
    context_stack: heapless::Vec<NodeContext, 16>,
    /// Path stack tracking the current path components from root
    path_stack: heapless::Vec<&'a str, 16>,
}

impl<'a> FdtIter<'a> {
    /// Creates a new FDT iterator from an FDT instance.
    ///
    /// Initializes the reader at the start of the structure block and the
    /// strings slice at the strings block. Also initializes the context
    /// stack with default values.
    pub fn new(fdt: Fdt<'a>) -> Self {
        let header = fdt.header();
        let struct_offset = header.off_dt_struct as usize;
        let strings_offset = header.off_dt_strings as usize;
        let strings_size = header.size_dt_strings as usize;

        let reader = fdt.data.reader_at(struct_offset);
        let strings = fdt
            .data
            .slice(strings_offset..strings_offset + strings_size);

        // Initialize context stack with default context
        let mut context_stack = heapless::Vec::new();
        let _ = context_stack.push(NodeContext::default());

        Self {
            fdt,
            reader,
            strings,
            node_iter: None,
            level: 0,
            finished: false,
            context_stack,
            path_stack: heapless::Vec::new(),
        }
    }

    /// Returns the current context (top of the stack).
    ///
    /// # Safety
    ///
    /// The stack is never empty because a default context is pushed on
    /// initialization in `FdtIter::new`.
    #[inline]
    fn current_context(&self) -> &NodeContext {
        // SAFETY: The stack is initialized with a default context and is never
        // completely emptied during iteration.
        self.context_stack.last().unwrap()
    }

    /// Handles an error by logging it and terminating iteration.
    ///
    /// When an error occurs during FDT parsing, we log it and stop iteration
    /// rather than panicking. This allows partial parsing and graceful degradation.
    fn handle_error(&mut self, err: FdtError) {
        error!("FDT parse error: {}", err);
        self.finished = true;
    }
}

impl<'a> Iterator for FdtIter<'a> {
    type Item = Node<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        loop {
            // If there's a node being processed, continue processing it
            if let Some(ref mut node_iter) = self.node_iter {
                match node_iter.process() {
                    Ok(OneNodeState::ChildBegin) => {
                        // Child node encountered, update reader position and clear current node iterator
                        self.reader = node_iter.reader().clone();
                        self.node_iter = None;
                        // Continue loop, next iteration will read BeginNode token
                    }
                    Ok(OneNodeState::End) => {
                        // Current node ended, update reader and decrease level
                        self.reader = node_iter.reader().clone();
                        self.node_iter = None;
                        if self.level > 0 {
                            self.level -= 1;
                            // Pop stack to restore parent node context
                            self.context_stack.pop();
                            self.path_stack.pop();
                        }
                        // Continue loop to process next token
                    }
                    Ok(OneNodeState::Processing) => {
                        // Should not reach here
                        continue;
                    }
                    Err(e) => {
                        self.handle_error(e);
                        return None;
                    }
                }
                continue;
            }

            // Read next token
            match self.reader.read_token() {
                Ok(Token::BeginNode) => {
                    // Create new node iterator to handle this node
                    let mut node_iter = OneNodeIter::new(
                        self.reader.clone(),
                        self.strings.clone(),
                        self.level,
                        self.current_context().clone(),
                        self.fdt.clone(),
                    );

                    // Read node name
                    match node_iter.read_node_name(&self.path_stack) {
                        Ok(mut node) => {
                            // Process node properties to get address-cells, size-cells
                            match node_iter.process() {
                                Ok(state) => {
                                    let props = node_iter.parsed_props();

                                    // Update node's cells
                                    node.address_cells = props.address_cells.unwrap_or(2);
                                    node.size_cells = props.size_cells.unwrap_or(1);

                                    // Decide next action based on state
                                    match state {
                                        OneNodeState::ChildBegin => {
                                            // Has child nodes, push child context
                                            let child_context = NodeContext {
                                                address_cells: node.address_cells,
                                                size_cells: node.size_cells,
                                            };
                                            let _ = self.context_stack.push(child_context);

                                            // Has child nodes, update reader position
                                            self.reader = node_iter.reader().clone();
                                            // Push current node name onto path stack
                                            if !node.name().is_empty() {
                                                let _ = self.path_stack.push(node.name());
                                            }
                                            // Increase level (node has children)
                                            self.level += 1;
                                        }
                                        OneNodeState::End => {
                                            // Node ended (no children), update reader
                                            self.reader = node_iter.reader().clone();
                                            // Don't push or update context since node has no children
                                            // Don't increase level since node is already closed
                                        }
                                        OneNodeState::Processing => {
                                            // Should not reach here, process() should always return ChildBegin or End
                                            self.node_iter = Some(node_iter);
                                            self.level += 1;
                                        }
                                    }

                                    return Some(node.into());
                                }
                                Err(e) => {
                                    self.handle_error(e);
                                    return None;
                                }
                            }
                        }
                        Err(e) => {
                            self.handle_error(e);
                            return None;
                        }
                    }
                }
                Ok(Token::EndNode) => {
                    // Top-level EndNode, decrease level
                    if self.level > 0 {
                        self.level -= 1;
                        // Pop stack to restore parent node context
                        self.context_stack.pop();
                        self.path_stack.pop();
                    }
                    continue;
                }
                Ok(Token::End) => {
                    // Structure block ended
                    self.finished = true;
                    return None;
                }
                Ok(Token::Nop) => {
                    // Ignore NOP tokens
                    continue;
                }
                Ok(Token::Prop) | Ok(Token::Data(_)) => {
                    // Property or unknown data at top level is an error
                    self.handle_error(FdtError::BufferTooSmall {
                        pos: self.reader.position(),
                    });
                    return None;
                }
                Err(e) => {
                    self.handle_error(e);
                    return None;
                }
            }
        }
    }
}
