use std::{
    future::ready,
    panic::{RefUnwindSafe, UnwindSafe},
};

use futures::stream;
use futures_concurrency::{
    future::{FutureGroup, Join, Race, RaceOk, TryJoin},
    stream::{Chain, Merge, StreamGroup, Zip},
};

fn assert_traits<T: Send + Sync + Unpin + UnwindSafe + RefUnwindSafe>(_: T) {}

#[test]
fn arrays() {
    assert_traits([ready(())].join());
    assert_traits([ready(())].race());
    assert_traits([ready(Ok::<_, ()>(()))].try_join());
    assert_traits([ready(Ok::<_, ()>(()))].race_ok());
    assert_traits([stream::iter([0])].chain());
    assert_traits([stream::iter([0])].merge());
    assert_traits([stream::iter([0])].zip());
}

#[test]
fn vectors() {
    assert_traits(vec![ready(())].join());
    assert_traits(vec![ready(())].race());
    assert_traits(vec![ready(Ok::<_, ()>(()))].try_join());
    assert_traits(vec![ready(Ok::<_, ()>(()))].race_ok());
    assert_traits(vec![stream::iter([0])].chain());
    assert_traits(vec![stream::iter([0])].merge());
    assert_traits(vec![stream::iter([0])].zip());
}

#[test]
fn groups() {
    assert_traits([ready(())].into_iter().collect::<FutureGroup<_>>());
    assert_traits([stream::iter([0])].into_iter().collect::<StreamGroup<_>>());
}
