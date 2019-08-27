#![feature(await_macro, async_await)]

#[cfg(test)]
#[macro_use]
mod support;

#[cfg(test)]
mod batch;
#[cfg(test)]
mod db_tests;
