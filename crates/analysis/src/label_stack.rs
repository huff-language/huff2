#[derive(Debug)]
pub struct LabelStack<'a, V> {
    context_sizes: Vec<usize>,
    label_stack: Vec<(&'a str, V)>,
}

impl<'a, V> LabelStack<'a, V> {
    pub fn new() -> Self {
        Self {
            context_sizes: Vec::with_capacity(16),
            label_stack: Vec::with_capacity(256),
        }
    }

    pub fn enter_context(&mut self) {
        self.context_sizes.push(self.label_stack.len());
    }

    pub fn leave_context(&mut self) -> Option<usize> {
        let previous_size = self.context_sizes.pop()?;
        self.label_stack.resize_with(previous_size, || {
            panic!("Stored size *larger* then stack size")
        });
        Some(previous_size)
    }

    pub fn insert(&mut self, label: &'a str, value: V) {
        self.label_stack.push((label, value));
    }

    pub fn get(&mut self, target_label: &'a str) -> Option<&V> {
        self.label_stack
            .iter()
            .rev()
            .filter_map(|(label, value)| {
                if *label == target_label {
                    Some(value)
                } else {
                    None
                }
            })
            .next()
    }

    pub fn contains(&mut self, label: &'a str) -> bool {
        self.get(label).is_some()
    }
}

impl<'a> LabelStack<'a, ()> {
    pub fn add(&mut self, label: &'a str) {
        self.insert(label, ());
    }
}

impl<V> Default for LabelStack<'_, V> {
    fn default() -> Self {
        Self::new()
    }
}