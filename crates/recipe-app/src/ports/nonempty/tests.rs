use super::{EmptyVec, NonEmpty};

#[test]
fn try_from_an_empty_vec_gives_empty_vec_error() {
    assert_eq!(NonEmpty::<u8>::try_from(Vec::new()), Err(EmptyVec));
}

#[test]
fn try_from_keeps_the_order() {
    let list = NonEmpty::try_from(vec![1, 2, 3]).unwrap();
    assert_eq!(Vec::from(list), vec![1, 2, 3]);
}

#[test]
fn with_last_puts_the_last_element_after_the_others() {
    let list = NonEmpty::with_last(vec!['a', 'b'], 'c');
    assert_eq!(Vec::from(list), vec!['a', 'b', 'c']);
}

#[test]
fn with_last_without_others_holds_only_the_last_element() {
    let list = NonEmpty::with_last(Vec::new(), 'z');
    assert_eq!(Vec::from(list), vec!['z']);
}

#[test]
fn new_and_push_keep_the_order() {
    let mut list = NonEmpty::new('a');
    list.push('b');
    list.push('c');
    assert_eq!(
        list.iter().copied().collect::<Vec<_>>(),
        vec!['a', 'b', 'c']
    );
    assert_eq!(Vec::from(list), vec!['a', 'b', 'c']);
}

#[test]
fn len_counts_the_first_element_and_the_rest() {
    let mut list = NonEmpty::new(10);
    assert_eq!(list.len(), 1);
    list.push(20);
    assert_eq!(list.len(), 2);
}

#[test]
fn first_gives_the_first_element() {
    let mut list = NonEmpty::new("x");
    list.push("y");
    assert_eq!(list.first(), &"x");
}

#[test]
fn a_list_is_never_empty() {
    assert!(!NonEmpty::new(0).is_empty());
}
