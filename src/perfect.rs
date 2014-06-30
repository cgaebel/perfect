//! Hashtables with no wasted space and no collisions.
//!
//! These tables are slow to initialize, but much faster than traditional
//! hashtables at lookup/insertion/deletion of elements that it knows about.
//! Elements that it doesn't know about are thrown into a backup traditional
//! hashtable. This hashtable is lazily initialized.
#![crate_type = "lib"]
#![feature(macro_rules, default_type_params, phase)]
#![deny(warnings, missing_doc)]
#[phase(plugin, link)] extern crate log;
extern crate graph;

use std::cmp;
use std::collections;
use std::hash;
use std::rand;
use std::vec;
use graph::Graph;

pub struct HashMap<K, V> {
  nodes:  Vec<uint>,
  t1:     Vec<uint>,
  t2:     Vec<uint>,
  table:  Vec<Option<(K, V)>>,
  backup: Option<collections::HashMap<K, V>>,
}

pub struct PerfectHashState<'a> {
  t1: &'a [uint],
  t2: &'a [uint],
  max_length: uint,
  n:  uint,
  m:  uint,
  i:  uint,
  u:  uint,
  v:  uint,
}

impl<'a> hash::Writer for PerfectHashState<'a> {
  fn write(&mut self, bytes: &[u8]) {
    for (&b, i) in bytes.iter().zip(range(self.i, self.max_length)) {
      let bu = b as uint;
      self.u = self.u.checked_add(&(self.t1[i] * bu)).expect("should not overflow");
      self.v = self.v.checked_add(&(self.t2[i] * bu)).expect("should not overflow");
    }
    self.i = cmp::min(self.i + bytes.len(), self.max_length);
  }
}

impl<'a> PerfectHashState<'a> {
  fn new<'a>(t1: &'a [uint], t2: &'a [uint], n: uint, m: uint) -> PerfectHashState<'a> {
    PerfectHashState {
      t1: t1,
      t2: t2,
      max_length: t1.len(),
      n: n,
      m: m,
      i: 0,
      u: 0,
      v: 0
    }
  }

  fn get_u(&self) -> uint {
    self.u % self.n
  }

  fn get_v(&self) -> uint {
    self.v % self.n
  }
}

struct ByteCounter {
  i: uint,
}

impl hash::Writer for ByteCounter {
  fn write(&mut self, bytes: &[u8]) {
    self.i += bytes.len();
  }
}

impl ByteCounter {
  fn new() -> ByteCounter {
    ByteCounter { i: 0 }
  }

  fn get_count(&self) -> uint {
    self.i
  }
}

fn gen_table<R: rand::Rng>(rng: &R, n: uint, m: uint) -> Vec<uint> {
  rng.gen_iter().map(|x: uint| x % n).take(m).collect()
}

impl<'a,
     K: Eq
      + hash::Hash
      + hash::Hash<PerfectHashState<'a>>
      + hash::Hash<ByteCounter>,
     V>
    HashMap<K, V> {

  pub fn new(known_vals: Vec<K>) -> HashMap<K, V> {
    let max_length = known_vals.iter().map(|k| {
        let mut c = ByteCounter::new();
        k.hash(&mut c);
        c.get_count()
      }).max();

    let mut rng = rand::task_rng();

    let m = known_vals.len();

    // c = 2.08 according to the paper. As long as it's greater than 2,
    // we're good.
    let n = 2*m + m/12;

    let acyclic_t1 : Vec<uint>;
    let acyclic_t2 : Vec<uint>;
    let acyclic_g  : Graph<(), ()>;

    let mut iters : uint = 0;

    loop {
      let g : Graph<(), ()> = Graph::new();

      let t1 = gen_table(&rng, n, m);
      let t2 = gen_table(&rng, n, m);

      for w in known_vals.iter() {
        let mut state = PerfectHashState::new(t1.as_slice(), t2.as_slice(), n, m);
        w.hash(&mut state);
        let f1 = state.get_u();
        let f2 = state.get_v();
        g.insert_vertex(f1, ());
        g.insert_vertex(f2, ());
        g.insert_directed_edge(f1, f2, ());
      }

      iters += 1;

      if g.is_acyclic() {
        acyclic_g = g;
        break;
      }
    }

    debug!("Number of iterations: {}", iters);
  }
}
