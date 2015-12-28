#![feature(test)]
#![feature(plugin)]
#![feature(zero_one)]
// #![plugin(quickcheck_macros)]

#[macro_use] extern crate log;


//
// cargo test listedit::experiments -- --nocapture
//

extern crate adapton ;
extern crate test;
extern crate quickcheck;
extern crate rand;

use std::num::Zero;
use std::ops::Add;    

use adapton::adapton_sigs::* ;
use adapton::collection_traits::*;
use adapton::collection_edit::*;
use adapton::collection::*;
use adapton::engine;
use adapton::naive;
use std::fmt::Debug;
use std::hash::Hash;

type Edits = Vec<CursorEdit<u32, Dir2>>;

fn has_consecutive_names<A:Adapton,X,L:ListT<A,X>> (st:&mut A, list:L::List) -> bool {
    L::elim(st, list,
            |st| false,
            |st,x,xs| has_consecutive_names::<A,X,L> (st, xs),
            |st,n,xs|
            L::elim(st, xs,
                    |st| false,
                    |st,y,ys| has_consecutive_names::<A,X,L> (st, ys),
                    |st,m,ys| true))
}

pub struct Experiment ;
impl<A:Adapton,X:Ord+Add<Output=X>+Zero+Hash+Debug+PartialEq+Eq+Clone+PartialOrd> ExperimentT<A,X,Vec<X>>
    for Experiment
{
    type ListEdit = ListZipper<A,X,List<A,X>> ;
    fn run (st:&mut A, edits:Vec<CursorEdit<X,Dir2>>, view:ListReduce) -> Vec<(Vec<X>,Cnt)> {
        debug!("run");
        let mut outs : Vec<(Vec<X>,Cnt)> = Vec::new();
        let mut z : ListZipper<A,X,List<A,X>> = Self::ListEdit::empty(st) ;
        let mut loop_cnt = 0 as usize;
        for edit in edits.into_iter() {
            debug!("\n----------------------- Loop head; count={}", loop_cnt);
            debug!("zipper: {:?}", z);
            if false {
                let consecutive_left  = has_consecutive_names::<A,X,List<A,X>>(st, z.left.clone());
                let consecutive_right = has_consecutive_names::<A,X,List<A,X>>(st, z.right.clone());
                debug!("zipper names: consecutive left: {}, consecutive right: {}",
                       consecutive_left, consecutive_right);
                assert!(!consecutive_left);  // Todo-Later: This assertion generally fails for random interactions
                assert!(!consecutive_right); // Todo-Later: This assertion generally fails for random interactions
            }
            debug!("edit:   {:?}", edit);
            let (out, cnt) = st.cnt(|st|{
                let z_next = eval_edit::<A,X,Self::ListEdit>(st, edit, z.clone(), loop_cnt);
                let tree = Self::ListEdit::get_tree::<Tree<A,X,u32>>(st, z_next.clone(), Dir2::Left);
                debug!("tree:   {:?}", tree);
                let nm = st.name_of_string("eval_reduce".to_string());
                let out = st.ns(nm, |st|eval_reduce::<A,X,List<A,X>,Tree<A,X,u32>>(st, tree, &view) );
                z = z_next;
                loop_cnt = loop_cnt + 1;
                out
            }) ;
            debug!("out:    {:?}", out);
            debug!("cnt:    {:?}", cnt);
            outs.push((out,cnt));
        } outs
    }
}

fn compare_naive_and_cached(edits: &Edits, view:&ListReduce) -> bool {
    let mut n_st = naive::AdaptonFromScratch::new();
    let mut e_st = engine::Engine::new();
    // e_st.flags.ignore_nominal_use_structural = true;
    
    let results_1 = Experiment::run(&mut n_st, edits.clone(), view.clone());
    let results_2 = Experiment::run(&mut e_st, edits.clone(), view.clone());
    
    let mut idx = 0;
    let mut a_cost : Cnt = Cnt::zero();
    let mut b_cost : Cnt = Cnt::zero();
    for (a, b) in results_1.iter().zip(results_2.iter()) {
        a_cost = &a_cost + &a.1 ;
        b_cost = &b_cost + &b.1 ;
        if a.0 != b.0 {
            println!("After edit {}, {:?}, expected {:?} to be {:?}, but found {:?}.\nEdits:\n{:?}",
                     idx, edits[idx], &view, a.0, b.0, edits);
            return false;
        }
        idx += 1;
    }
    {
        let naive_total = a_cost.eval ;
        let engine_total = b_cost.dirty + b_cost.eval + b_cost.change_prop ;
        if false {
        println!("{:16} for {:5} edits, Naive/Engine:{:5} = {:8} / {:8}. Naive/EngineEval:{:5}. In Engine, eval is {:.2} of {:?}",
                 format!("{:?}", view),
                 edits.len(),
                 (naive_total as f32) / (engine_total as f32),
                 naive_total, engine_total,
                 (naive_total as f32) / (b_cost.eval as f32),
                 (b_cost.eval as f32) / (engine_total as f32),
                 b_cost);
        } ;
        println!("{:24} For {:5} edits, Naive/Engine:{:5}, Naive/EngineEval:{:5} \t==> Per-edit ==> Naive:{:8}, Engine:{:6}, EngineEval:{:5},   Naive/Engine:{:5}, Naive/EngineEval:{:5}",
                 format!("{:?}", view),
                 edits.len(),
                 format!("{:.2}", (naive_total as f32) / (engine_total as f32)),
                 format!("{:.2}", (naive_total as f32) / (b_cost.eval as f32)),
                 // Per-edit metrics:
                 format!("{:.2}", (naive_total as f32) / (edits.len() as f32)),
                 format!("{:.2}", (engine_total as f32) / (edits.len() as f32)),
                 format!("{:.2}", (b_cost.eval as f32) / (edits.len() as f32)),
                 format!("{:.2}", ((naive_total as f32) / (engine_total as f32)) / (edits.len() as f32)),
                 format!("{:.2}", ((naive_total as f32) / (b_cost.eval as f32)) / (edits.len() as f32)),
                 );
    }
    true
}

fn ensure_consistency_randomly(size:usize, iterations:usize, view:&ListReduce) {
    let rng = rand::thread_rng();
    let mut gen = quickcheck::StdGen::new(rng, size);
    for _ in 0..iterations {
        let testv = Box::new(<Edits as quickcheck::Arbitrary>::arbitrary(&mut gen));
        assert!( compare_naive_and_cached(&*testv, view) )
    }
}

#[test]
fn ensure_consistency_randomly_100_x_100() {
    //ensure_consistency_randomly(100, 100, &ListReduce::Sum) ;
    //ensure_consistency_randomly(100, 100, &ListReduce::Max) ;
    //ensure_consistency_randomly(100, 100, &ListReduce::DemandAll(ListTransf::Reverse)) ;
    ensure_consistency_randomly(100, 100, &ListReduce::DemandAll(ListTransf::Sort)) ;
}

#[test]
fn ensure_consistency_randomly_300_x_100() {
    //ensure_consistency_randomly(300, 100, &ListReduce::Sum) ;
    //ensure_consistency_randomly(300, 100, &ListReduce::Max) ;
    //ensure_consistency_randomly(300, 100, &ListReduce::DemandAll(ListTransf::Reverse)) ;
    ensure_consistency_randomly(300, 100, &ListReduce::DemandAll(ListTransf::Sort)) ;
}

#[test]
fn ensure_consistency_randomly_1k_x_20() {
    ensure_consistency_randomly(1000, 20, &ListReduce::Sum) ;
    ensure_consistency_randomly(1000, 20, &ListReduce::Max) ;
    ensure_consistency_randomly(1000, 20, &ListReduce::DemandAll(ListTransf::Reverse)) ;
    ensure_consistency_randomly(1000, 20, &ListReduce::DemandAll(ListTransf::Sort)) ;
}

#[test]
fn ensure_consistency_randomly_5k_x_5() {
    ensure_consistency_randomly(5000, 5, &ListReduce::Sum) ;
    ensure_consistency_randomly(5000, 5, &ListReduce::Max)
}

#[test]
fn ensure_consistency_randomly_10k_x_5() {
    ensure_consistency_randomly(10000, 5, &ListReduce::Sum) ;
    ensure_consistency_randomly(10000, 5, &ListReduce::Max)
}

// Nominal
// After edit 23, Insert(Right, 60), expected DemandAll(Sort) to be [6, 9, 10, 16, 21, 29, 44, 58, 60, 62, 91], but found [6, 9, 10, 16, 21, 29, 44, 58, 62, 91].
// Edits:
// [Goto(Right), Insert(Right, 44), Insert(Left, 9), Goto(Right), Goto(Right), Insert(Left, 91), Insert(Right, 29), Insert(Right, 62), Goto(Right), Insert(Right, 71), Remove(Right), Insert(Right, 87), Insert(Left, 4), Remove(Right), Insert(Right, 19), Replace(Left, 21), Insert(Left, 16), Goto(Right), Insert(Right, 6), Insert(Right, 10), Goto(Left), Remove(Right), Insert(Left, 58), Insert(Right, 60), Insert(Right, 86), Insert(Right, 36), Insert(Right, 20), Goto(Right), Insert(Right, 94)]

// Nominal
// After edit 23, Replace(Right, 75), expected DemandAll(Sort) to be [10, 13, 19, 21, 56, 61, 68, 75, 75, 92], but found [10, 13, 19, 21, 48, 56, 61, 68, 75, 92].
//    Edits:
// [Insert(Left, 92), Insert(Left, 99), Insert(Right, 56), Insert(Right, 64), Remove(Left), Insert(Right, 82), Insert(Right, 10), Insert(Left, 22), Goto(Right), Replace(Right, 13), Remove(Left), Goto(Right), Insert(Left, 19), Replace(Right, 10), Goto(Left), Insert(Left, 68), Goto(Right), Insert(Right, 21), Insert(Right, 61), Insert(Right, 48), Goto(Left), Goto(Right), Insert(Left, 75), Replace(Right, 75), Insert(Right, 75), Goto(Right), Insert(Left, 10), Replace(Left, 96), Insert(Left, 86), Insert(Right, 42), Insert(Right, 82), Replace(Right, 38), Remove(Left), Remove(Left), Goto(Right), Insert(Right, 9), Replace(Left, 8), Remove(Right), Replace(Left, 4), Insert(Right, 69), Insert(Right, 40), Goto(Right), Insert(Right, 31), Goto(Right), Insert(Right, 31), Insert(Left, 26), Insert(Right, 92), Insert(Left, 46), Goto(Right), Insert(Right, 98), Insert(Left, 53), Insert(Left, 0), Goto(Right), Insert(Right, 56), Insert(Left, 32), Insert(Left, 0), Replace(Right, 20), Insert(Right, 95), Goto(Right), Insert(Left, 38), Insert(Left, 81), Insert(Right, 79), Insert(Left, 40), Goto(Right), Insert(Left, 27), Insert(Left, 27), Insert(Left, 88), Insert(Left, 37), Goto(Right), Insert(Left, 69)]

// Nominal
// After edit 36, Insert(Left, 32), expected DemandAll(Sort) to be [1, 26, 31, 32, 55, 64, 65, 79, 81, 85, 88], but found [1, 26, 31, 55, 64, 65, 79, 81, 85, 88].
// Edits:
// [Insert(Left, 60), Goto(Right), Insert(Left, 25), Remove(Left), Replace(Left, 86), Goto(Left), Goto(Right), Remove(Right), Replace(Left, 81), Goto(Right), Goto(Right), Remove(Right), Goto(Right), Insert(Left, 85), Insert(Right, 10), Insert(Right, 53), Goto(Right), Replace(Left, 64), Replace(Right, 23), Insert(Left, 66), Remove(Left), Insert(Left, 1), Insert(Right, 17), Insert(Left, 9), Replace(Left, 31), Insert(Right, 32), Insert(Left, 76), Replace(Left, 58), Replace(Left, 55), Remove(Right), Remove(Right), Replace(Right, 26), Insert(Right, 88), Insert(Left, 79), Insert(Left, 65), Goto(Right), Insert(Left, 32)]

// Nominal
// After edit 33, Insert(Left, 36), expected DemandAll(Sort) to be [2, 6, 18, 21, 26, 31, 35, 35, 36, 36, 43, 66, 69, 72, 72, 77, 80, 94], but found [2, 6, 18, 21, 26, 31, 35, 35, 36, 43, 66, 69, 72, 72, 77, 80, 94].
//    Edits:
// [Insert(Left, 35), Insert(Left, 94), Insert(Left, 2), Goto(Right), Insert(Left, 56), Goto(Right), Replace(Left, 43), Goto(Right), Goto(Right), Goto(Right), Insert(Left, 66), Insert(Right, 22), Goto(Right), Goto(Right), Replace(Left, 6), Insert(Right, 26), Goto(Right), Insert(Right, 35), Insert(Right, 36), Goto(Left), Insert(Left, 31), Insert(Right, 6), Insert(Right, 18), Goto(Right), Replace(Right, 77), Goto(Right), Insert(Right, 72), Insert(Right, 69), Insert(Right, 80), Insert(Right, 72), Insert(Left, 72), Insert(Left, 21), Remove(Right), Insert(Left, 36), Remove(Left), Insert(Right, 63), Insert(Left, 30), Remove(Right), Remove(Left), Insert(Left, 14)]

// Nominal
// After edit 33, Insert(Right, 52), expected DemandAll(Sort) to be [21, 28, 29, 41, 52, 54, 56, 56, 57, 63, 66, 67, 67, 72, 81], but found [21, 28, 29, 41, 54, 56, 56, 57, 63, 66, 67, 67, 72, 81].
// Edits:
// [Goto(Left), Insert(Left, 93), Remove(Left), Insert(Left, 38), Insert(Left, 70), Remove(Right), Remove(Left), Replace(Right, 26), Remove(Left), Insert(Right, 53), Insert(Left, 54), Insert(Right, 8), Insert(Left, 56), Insert(Left, 57), Remove(Right), Insert(Left, 67), Remove(Right), Insert(Left, 67), Insert(Left, 41), Insert(Right, 56), Goto(Right), Insert(Left, 63), Insert(Right, 24), Remove(Right), Remove(Right), Insert(Left, 21), Insert(Right, 28), Insert(Right, 81), Insert(Left, 72), Insert(Left, 66), Insert(Right, 29), Goto(Right), Goto(Right), Insert(Right, 52), Insert(Right, 80), Insert(Left, 3), Goto(Right), Insert(Left, 75), Remove(Left), Replace(Left, 69), Insert(Right, 47), Insert(Left, 75), Insert(Right, 28), Goto(Right), Insert(Right, 68), Insert(Right, 5), Goto(Right)]

// Nominal
// After edit 46, Replace(Left, 93), expected Sum to be [1490], but found [1397].
//     thread 'ensure_consistency_randomly_100_x_100' panicked at '[Goto(Left), Insert(Right, 93), Insert(Right, 50), Insert(Right, 82), Goto(Right), Insert(Right, 79), Insert(Right, 79), Goto(Right), Goto(Right), Goto(Right), Remove(Right), Insert(Right, 6), Insert(Right, 89), Insert(Left, 45), Replace(Right, 89), Insert(Right, 19), Insert(Left, 55), Insert(Right, 47), Insert(Right, 41), Insert(Left, 83), Insert(Right, 40), Goto(Right), Insert(Right, 84), Insert(Right, 90), Goto(Right), Insert(Right, 95), Insert(Right, 60), Insert(Left, 96), Insert(Right, 80), Goto(Right), Insert(Left, 33), Goto(Right), Goto(Left), Replace(Right, 11), Insert(Left, 94), Insert(Left, 0), Goto(Right), Goto(Right), Insert(Left, 91), Insert(Left, 24), Replace(Left, 8), Goto(Left), Insert(Right, 0), Goto(Right), Insert(Left, 91), Remove(Left), Replace(Left, 93), Insert(Right, 23), Insert(Right, 38), Insert(Right, 3), Insert(Right, 51), Goto(Right), Replace(Right, 58), Insert(Left, 53), Insert(Left, 90), Goto(Right), Goto(Right), Goto(Right), Insert(Right, 16), Replace(Right, 9), Remove(Left), Goto(Right), Remove(Right), Remove(Left), Goto(Right), Remove(Left), Insert(Left, 48), Insert(Left, 39), Goto(Right), Insert(Left, 75), Insert(Right, 26), Goto(Right), Goto(Right), Replace(Left, 92), Replace(Right, 5), Goto(Right), Insert(Left, 97), Insert(Right, 53), Remove(Right)]', tests/listedit.rs:135

// Nominal
// After edit 185, Insert(Left, 120), expected Sum to be [7978], but found [7858].
// thread 'ensure_consistency_randomly_300_x_100' panicked at '[Insert(Left, 119), Goto(Right), Remove(Left), Replace(Right, 213), Goto(Right), Replace(Left, 209), Insert(Right, 245), Insert(Right, 66), Insert(Left, 281), Remove(Right), Insert(Right, 29), Insert(Right, 4), Goto(Right), Insert(Left, 237), Insert(Left, 57), Goto(Right), Goto(Right), Insert(Left, 112), Remove(Right), Insert(Left, 61), Goto(Right), Remove(Right), Goto(Right), Replace(Left, 108), Insert(Right, 132), Goto(Right), Remove(Left), Remove(Left), Goto(Right), Replace(Left, 151), Goto(Right), Insert(Left, 127), Goto(Left), Goto(Left), Insert(Left, 91), Insert(Right, 115), Goto(Right), Insert(Right, 189), Insert(Right, 261), Insert(Right, 5), Insert(Left, 267), Goto(Right), Insert(Left, 269), Goto(Right), Insert(Right, 9), Insert(Right, 104), Goto(Right), Remove(Left), Insert(Left, 209), Goto(Right), Insert(Left, 29), Insert(Left, 196), Goto(Right), Insert(Left, 92), Goto(Left), Replace(Right, 28), Insert(Left, 260), Replace(Right, 289), Goto(Left), Insert(Left, 164), Goto(Right), Goto(Left), Insert(Left, 236), Remove(Right), Remove(Right), Replace(Left, 47), Insert(Right, 111), Insert(Right, 59), Insert(Right, 167), Goto(Right), Goto(Right), Insert(Left, 129), Replace(Right, 46), Remove(Left), Replace(Right, 109), Insert(Left, 266), Remove(Left), Replace(Left, 257), Remove(Right), Goto(Right), Insert(Left, 92), Insert(Left, 202), Insert(Left, 184), Insert(Left, 98), Insert(Left, 217), Insert(Right, 197), Insert(Right, 219), Remove(Right), Insert(Right, 9), Goto(Right), Goto(Right), Goto(Right), Goto(Left), Insert(Left, 147), Goto(Right), Insert(Right, 141), Insert(Right, 171), Replace(Right, 138), Insert(Right, 38), Insert(Right, 27), Remove(Right), Remove(Right), Goto(Right), Goto(Right), Remove(Left), Insert(Right, 266), Goto(Right), Goto(Right), Insert(Left, 22), Insert(Right, 93), Goto(Right), Insert(Left, 117), Goto(Right), Insert(Right, 233), Remove(Left), Insert(Left, 7), Insert(Left, 103), Insert(Right, 111), Goto(Right), Goto(Right), Insert(Left, 254), Remove(Right), Remove(Left), Goto(Right), Insert(Left, 297), Goto(Right), Remove(Left), Goto(Right), Replace(Right, 97), Insert(Left, 206), Goto(Right), Insert(Left, 121), Insert(Left, 80), Insert(Left, 63), Insert(Left, 145), Insert(Right, 156), Insert(Left, 80), Insert(Left, 224), Remove(Left), Insert(Right, 82), Remove(Right), Goto(Right), Remove(Right), Goto(Right), Goto(Right), Insert(Right, 85), Insert(Left, 294), Replace(Right, 37), Remove(Right), Replace(Left, 71), Goto(Right), Goto(Right), Insert(Right, 35), Goto(Right), Insert(Left, 172), Goto(Right), Goto(Right), Remove(Right), Goto(Right), Goto(Right), Insert(Left, 205), Insert(Right, 0), Replace(Left, 181), Insert(Right, 133), Remove(Right), Insert(Left, 108), Remove(Right), Insert(Right, 171), Insert(Left, 247), Insert(Left, 259), Goto(Left), Insert(Left, 218), Insert(Left, 12), Goto(Right), Remove(Right), Insert(Right, 198), Replace(Right, 96), Goto(Right), Remove(Right), Goto(Right), Goto(Right), Insert(Left, 83), Replace(Right, 37), Insert(Left, 214), Remove(Left), Insert(Left, 120), Replace(Right, 279), Insert(Right, 78), Replace(Left, 243), Remove(Left), Replace(Left, 216), Goto(Right), Goto(Right), Replace(Right, 293), Goto(Right), Goto(Right), Remove(Right), Remove(Right), Remove(Left), Goto(Right), Replace(Left, 115), Goto(Right), Insert(Left, 274), Remove(Right), Insert(Right, 76), Goto(Left), Remove(Right), Insert(Right, 233), Insert(Right, 167), Goto(Right), Remove(Left), Insert(Right, 41), Insert(Right, 138), Insert(Right, 25), Goto(Left), Insert(Right, 251), Insert(Right, 22), Goto(Right), Remove(Left), Insert(Right, 250), Goto(Right), Insert(Right, 139), Goto(Left), Replace(Right, 147), Insert(Left, 56), Insert(Right, 240), Insert(Left, 125), Insert(Left, 46), Goto(Right), Insert(Left, 116), Insert(Right, 203), Insert(Right, 132), Insert(Right, 188), Goto(Right), Insert(Left, 244), Remove(Left), Insert(Left, 263), Replace(Right, 80), Insert(Left, 299), Insert(Right, 212), Goto(Left), Replace(Right, 241), Insert(Left, 291), Goto(Right), Goto(Right)]', tests/listedit.rs:138
//    test ensure_consistency_randomly_300_x_100 ... FAILED

// Structural
// After edit 45, Goto(Right), expected Sum to be [952], but found [865].
// thread 'ensure_consistency_randomly_100_x_100' panicked at '[Insert(Right, 37), Goto(Right), Remove(Right), Replace(Right, 99), Insert(Left, 16), Goto(Right), Insert(Left, 76), Goto(Right), Insert(Left, 60), Goto(Right), Replace(Left, 33), Insert(Right, 60), Insert(Left, 80), Insert(Right, 93), Insert(Left, 49), Replace(Right, 68), Insert(Left, 27), Insert(Left, 82), Insert(Left, 11), Goto(Right), Insert(Left, 37), Insert(Right, 7), Goto(Right), Replace(Right, 28), Goto(Right), Insert(Left, 3), Replace(Left, 32), Goto(Right), Goto(Right), Insert(Left, 69), Replace(Right, 11), Insert(Right, 67), Replace(Right, 31), Insert(Left, 83), Replace(Right, 61), Replace(Right, 60), Goto(Right), Goto(Right), Insert(Right, 69), Insert(Right, 70), Goto(Right), Remove(Right), Insert(Left, 66), Remove(Left), Insert(Right, 87), Goto(Right), Replace(Left, 71), Insert(Left, 34), Replace(Right, 93), Insert(Right, 91), Remove(Right), Insert(Left, 69), Goto(Right), Goto(Right), Insert(Right, 49), Insert(Left, 82), Insert(Left, 8), Replace(Left, 51), Insert(Right, 54), Insert(Left, 19), Goto(Right), Replace(Right, 18), Replace(Right, 63), Remove(Left), Remove(Left), Remove(Left), Goto(Right), Replace(Left, 28), Insert(Left, 88), Insert(Left, 18), Insert(Left, 76), Remove(Left), Goto(Right), Goto(Right), Replace(Left, 83), Goto(Right), Goto(Right), Insert(Left, 83), Insert(Left, 55), Insert(Right, 71), Insert(Left, 1), Goto(Right), Goto(Right), Goto(Right), Insert(Left, 5), Insert(Left, 54), Insert(Right, 29), Insert(Left, 9), Replace(Right, 81), Insert(Right, 55), Insert(Left, 81), Insert(Left, 5), Insert(Right, 13), Goto(Right), Goto(Right), Insert(Left, 49), Insert(Right, 82), Insert(Left, 11)]', tests/listedit.rs:136

// Structural
// After edit 53, Goto(Right), expected Sum to be [1110], but found [1066].
// thread 'ensure_consistency_randomly_100_x_100' panicked at '[Goto(Right), Insert(Right, 18), Insert(Right, 22), Insert(Left, 47), Insert(Right, 32), Insert(Right, 96), Replace(Left, 37), Goto(Left), Goto(Right), Remove(Left), Insert(Left, 88), Insert(Left, 45), Insert(Left, 12), Insert(Left, 6), Insert(Right, 55), Insert(Right, 14), Insert(Right, 34), Replace(Right, 70), Insert(Left, 73), Insert(Left, 56), Insert(Left, 35), Insert(Left, 19), Remove(Right), Insert(Left, 73), Goto(Right), Replace(Right, 12), Goto(Right), Insert(Left, 5), Insert(Right, 50), Remove(Left), Remove(Left), Replace(Left, 44), Goto(Right), Insert(Left, 37), Goto(Right), Insert(Right, 12), Goto(Right), Goto(Right), Goto(Right), Replace(Left, 66), Insert(Left, 50), Insert(Left, 69), Insert(Right, 73), Goto(Right), Insert(Left, 45), Insert(Left, 18), Goto(Right), Replace(Right, 98), Insert(Left, 49), Goto(Right), Insert(Left, 66), Remove(Left), Insert(Right, 44), Goto(Right), Goto(Right), Insert(Left, 20), Insert(Right, 58), Insert(Left, 28), Insert(Right, 82), Goto(Right), Insert(Right, 7), Insert(Right, 87), Insert(Left, 97), Replace(Right, 16), Insert(Left, 79), Insert(Left, 82), Insert(Left, 92), Insert(Right, 91), Insert(Right, 6), Goto(Left), Replace(Right, 44), Insert(Left, 63), Insert(Right, 50), Goto(Left)]', tests/listedit.rs:136

// Structural
// After edit 43, Goto(Right), expected Sum to be [732], but found [717].
// thread 'ensure_consistency_randomly_100_x_100' panicked at '[Goto(Right), Insert(Left, 73), Remove(Left), Replace(Right, 2), Insert(Right, 62), Goto(Right), Remove(Right), Goto(Right), Goto(Right), Insert(Right, 75), Goto(Right), Goto(Left), Goto(Right), Goto(Right), Replace(Right, 40), Goto(Right), Remove(Right), Insert(Left, 6), Insert(Left, 95), Remove(Right), Remove(Right), Goto(Right), Insert(Right, 58), Insert(Left, 0), Insert(Left, 12), Goto(Right), Insert(Left, 41), Insert(Left, 93), Insert(Left, 81), Replace(Left, 47), Insert(Right, 42), Insert(Right, 67), Goto(Right), Goto(Right), Insert(Left, 3), Insert(Right, 42), Insert(Left, 74), Goto(Right), Goto(Right), Replace(Right, 60), Goto(Left), Goto(Right), Insert(Right, 15), Goto(Right), Insert(Right, 21), Goto(Right), Insert(Left, 90), Goto(Left), Insert(Left, 84), Insert(Right, 34), Goto(Left), Goto(Right), Goto(Right), Remove(Right), Goto(Right), Insert(Right, 66), Insert(Left, 49), Insert(Right, 62), Goto(Right), Insert(Right, 82), Insert(Right, 87), Insert(Right, 43), Goto(Right), Goto(Right), Insert(Right, 69)]', tests/listedit.rs:136

// -----------------------------------------------------------------------------------------------------
// Max Regression tests
//

#[test]
fn ensure_consistency_regression_testcase1() { assert!( compare_naive_and_cached(&testcase1(), &ListReduce::Max)) }

#[test]
fn ensure_consistency_regression_testcase2() { assert!( compare_naive_and_cached(&testcase2(), &ListReduce::Max)) }

#[test]
fn ensure_consistency_regression_testcase3() { assert!( compare_naive_and_cached(&testcase3(), &ListReduce::Max)) }

#[test]
fn ensure_consistency_regression_testcase4() { assert!( compare_naive_and_cached(&testcase4(), &ListReduce::Max)) }

#[test]
fn ensure_consistency_regression_testcase5() { assert!( compare_naive_and_cached(&testcase5(), &ListReduce::Max)) }

fn testcase1 () -> Edits {
    vec![
        CursorEdit::Insert(Dir2::Left, 36), CursorEdit::Insert(Dir2::Right, 44), CursorEdit::Remove(Dir2::Right), CursorEdit::Remove(Dir2::Right),
        CursorEdit::Insert(Dir2::Left, 86), CursorEdit::Insert(Dir2::Left, 11), CursorEdit::Insert(Dir2::Right, 22), CursorEdit::Insert(Dir2::Right, 23),
        CursorEdit::Insert(Dir2::Left, 41), CursorEdit::Remove(Dir2::Right),
        CursorEdit::Insert(Dir2::Right, 13), CursorEdit::Insert(Dir2::Left, 21), CursorEdit::Goto(Dir2::Right), CursorEdit::Insert(Dir2::Right, 41),
        CursorEdit::Insert(Dir2::Left, 71), CursorEdit::Goto(Dir2::Right), CursorEdit::Insert(Dir2::Right, 22), CursorEdit::Replace(Dir2::Left, 11), CursorEdit::Goto(Dir2::Right),
        CursorEdit::Insert(Dir2::Right, 76), CursorEdit::Insert(Dir2::Left, 45), CursorEdit::Goto(Dir2::Left), CursorEdit::Insert(Dir2::Left, 12), CursorEdit::Insert(Dir2::Right, 14),
        CursorEdit::Goto(Dir2::Right), CursorEdit::Goto(Dir2::Right), CursorEdit::Insert(Dir2::Right, 35), CursorEdit::Goto(Dir2::Right), CursorEdit::Insert(Dir2::Left, 39),
        CursorEdit::Goto(Dir2::Left), CursorEdit::Insert(Dir2::Right, 43), CursorEdit::Goto(Dir2::Right), CursorEdit::Insert(Dir2::Right, 36), CursorEdit::Insert(Dir2::Left, 85),
        CursorEdit::Insert(Dir2::Left, 11), CursorEdit::Insert(Dir2::Left, 93), CursorEdit::Insert(Dir2::Right, 52), CursorEdit::Goto(Dir2::Left), CursorEdit::Goto(Dir2::Right)
       ]
}

fn testcase2 () -> Edits {
    vec![CursorEdit::Goto(Dir2::Left), CursorEdit::Insert(Dir2::Left, 86), CursorEdit::Insert(Dir2::Left, 76), CursorEdit::Insert(Dir2::Right, 39), CursorEdit::Goto(Dir2::Right),
         CursorEdit::Insert(Dir2::Right, 63), CursorEdit::Goto(Dir2::Right), CursorEdit::Replace(Dir2::Left, 54), CursorEdit::Remove(Dir2::Left),
         CursorEdit::Insert(Dir2::Right, 77), CursorEdit::Insert(Dir2::Right, 32), CursorEdit::Goto(Dir2::Left), CursorEdit::Goto(Dir2::Left),
         CursorEdit::Replace(Dir2::Right, 57), CursorEdit::Goto(Dir2::Right), CursorEdit::Replace(Dir2::Left, 6), CursorEdit::Remove(Dir2::Right),
         CursorEdit::Goto(Dir2::Right), CursorEdit::Insert(Dir2::Left, 81), CursorEdit::Goto(Dir2::Right), CursorEdit::Goto(Dir2::Left),
         CursorEdit::Remove(Dir2::Right), CursorEdit::Goto(Dir2::Left), CursorEdit::Goto(Dir2::Left), CursorEdit::Goto(Dir2::Right),
         CursorEdit::Insert(Dir2::Right, 76), CursorEdit::Replace(Dir2::Left, 72), CursorEdit::Insert(Dir2::Left, 51), CursorEdit::Remove(Dir2::Right), CursorEdit::Insert(Dir2::Left, 49),
         CursorEdit::Remove(Dir2::Left), CursorEdit::Goto(Dir2::Right), CursorEdit::Goto(Dir2::Right), CursorEdit::Insert(Dir2::Left, 6),
         CursorEdit::Insert(Dir2::Right, 82), CursorEdit::Goto(Dir2::Right), CursorEdit::Insert(Dir2::Right, 89), CursorEdit::Goto(Dir2::Right),
         CursorEdit::Insert(Dir2::Left, 9), CursorEdit::Goto(Dir2::Left), CursorEdit::Insert(Dir2::Left, 26), CursorEdit::Replace(Dir2::Left, 35),
         CursorEdit::Goto(Dir2::Left), CursorEdit::Remove(Dir2::Left), CursorEdit::Goto(Dir2::Right), CursorEdit::Goto(Dir2::Left), CursorEdit::Goto(Dir2::Left),
         CursorEdit::Remove(Dir2::Left), CursorEdit::Goto(Dir2::Right), CursorEdit::Insert(Dir2::Left, 5), CursorEdit::Insert(Dir2::Left, 75),
         CursorEdit::Goto(Dir2::Left), CursorEdit::Insert(Dir2::Left, 32), CursorEdit::Replace(Dir2::Right, 74), CursorEdit::Insert(Dir2::Left, 77),
         CursorEdit::Insert(Dir2::Left, 71), CursorEdit::Insert(Dir2::Left, 44), CursorEdit::Goto(Dir2::Left), CursorEdit::Insert(Dir2::Right, 81),
         CursorEdit::Insert(Dir2::Left, 61), CursorEdit::Insert(Dir2::Right, 92), CursorEdit::Insert(Dir2::Left, 68), CursorEdit::Replace(Dir2::Right, 42),
         CursorEdit::Insert(Dir2::Right, 81), CursorEdit::Goto(Dir2::Right), CursorEdit::Goto(Dir2::Right), CursorEdit::Insert(Dir2::Right, 31),
         CursorEdit::Insert(Dir2::Right, 72), CursorEdit::Insert(Dir2::Right, 70), CursorEdit::Insert(Dir2::Left, 87), CursorEdit::Insert(Dir2::Right, 95),
         CursorEdit::Goto(Dir2::Left), CursorEdit::Goto(Dir2::Left), CursorEdit::Replace(Dir2::Right, 96), CursorEdit::Goto(Dir2::Right),
         CursorEdit::Goto(Dir2::Left), CursorEdit::Remove(Dir2::Left), CursorEdit::Goto(Dir2::Left)]
}

fn testcase3 () -> Edits {
    vec![CursorEdit::Insert(Dir2::Left, 86),
         CursorEdit::Insert(Dir2::Right, 37),
         CursorEdit::Remove(Dir2::Left),
         CursorEdit::Insert(Dir2::Left, 42),
         CursorEdit::Goto(Dir2::Right),
         CursorEdit::Insert(Dir2::Left, 68),
         CursorEdit::Insert(Dir2::Right, 18),
         CursorEdit::Remove(Dir2::Left),
         CursorEdit::Remove(Dir2::Right),
         CursorEdit::Goto(Dir2::Right),
         CursorEdit::Replace(Dir2::Left, 68),
         CursorEdit::Insert(Dir2::Left, 82),
         CursorEdit::Insert(Dir2::Right, 30),
         CursorEdit::Goto(Dir2::Right),
         CursorEdit::Insert(Dir2::Left, 78),
         CursorEdit::Insert(Dir2::Right, 88),
         CursorEdit::Insert(Dir2::Left, 38),
         CursorEdit::Insert(Dir2::Right, 91),
         CursorEdit::Goto(Dir2::Left),
         CursorEdit::Insert(Dir2::Right, 67),
         CursorEdit::Insert(Dir2::Right, 3),
         CursorEdit::Remove(Dir2::Right),
         CursorEdit::Insert(Dir2::Right, 16),
         CursorEdit::Insert(Dir2::Left, 4),
         CursorEdit::Insert(Dir2::Right, 29),
         CursorEdit::Insert(Dir2::Right, 92),
         CursorEdit::Insert(Dir2::Left, 79),
         CursorEdit::Replace(Dir2::Left, 88),
         CursorEdit::Goto(Dir2::Right),
         CursorEdit::Goto(Dir2::Left),
         // CursorEdit::Insert(Dir2::Left, 25), CursorEdit::Insert(Dir2::Left, 46), CursorEdit::Goto(Dir2::Left), CursorEdit::Goto(Dir2::Left),
         // CursorEdit::Insert(Dir2::Left, 18), CursorEdit::Insert(Dir2::Left, 1), CursorEdit::Insert(Dir2::Right, 43), CursorEdit::Goto(Dir2::Left), CursorEdit::Remove(Dir2::Left),
         // CursorEdit::Insert(Dir2::Right, 93), CursorEdit::Goto(Dir2::Left), CursorEdit::Insert(Dir2::Right, 10), CursorEdit::Remove(Dir2::Left),
         // CursorEdit::Insert(Dir2::Right, 34), CursorEdit::Remove(Dir2::Left), CursorEdit::Replace(Dir2::Left, 47), CursorEdit::Goto(Dir2::Right),
         // CursorEdit::Insert(Dir2::Left, 56), CursorEdit::Insert(Dir2::Left, 36), CursorEdit::Replace(Dir2::Right, 99), CursorEdit::Insert(Dir2::Right, 19),
         // CursorEdit::Insert(Dir2::Right, 35), CursorEdit::Goto(Dir2::Right), CursorEdit::Insert(Dir2::Right, 94), CursorEdit::Replace(Dir2::Right, 58), CursorEdit::Goto(Dir2::Right),
         // CursorEdit::Insert(Dir2::Left, 71)
         ]
}

fn testcase4 () -> Edits {
    vec![
        CursorEdit::Replace(Dir2::Left, 29),
        CursorEdit::Replace(Dir2::Left, 34),
        CursorEdit::Goto(Dir2::Right),
        CursorEdit::Goto(Dir2::Right),
        CursorEdit::Insert(Dir2::Right, 30),
        CursorEdit::Insert(Dir2::Left, 26),
        CursorEdit::Insert(Dir2::Right, 26),
        CursorEdit::Insert(Dir2::Left, 7),
        CursorEdit::Remove(Dir2::Left),
        CursorEdit::Goto(Dir2::Left),
        CursorEdit::Insert(Dir2::Left, 19),
        CursorEdit::Insert(Dir2::Left, 16),
        CursorEdit::Goto(Dir2::Right),
        CursorEdit::Goto(Dir2::Left),
        CursorEdit::Insert(Dir2::Right, 27),
        CursorEdit::Insert(Dir2::Right, 3),
        CursorEdit::Insert(Dir2::Left, 13),
        CursorEdit::Goto(Dir2::Left),
        CursorEdit::Insert(Dir2::Left, 26),
        CursorEdit::Insert(Dir2::Left, 10),
        CursorEdit::Insert(Dir2::Right, 2),
        CursorEdit::Insert(Dir2::Right, 38),
        CursorEdit::Insert(Dir2::Left, 36),
        CursorEdit::Replace(Dir2::Left, 36),
        CursorEdit::Insert(Dir2::Left, 8),
        CursorEdit::Insert(Dir2::Left, 39),
        CursorEdit::Replace(Dir2::Right, 7),
        CursorEdit::Insert(Dir2::Right, 30),
        CursorEdit::Goto(Dir2::Left),
        CursorEdit::Goto(Dir2::Right),
        //CursorEdit::Goto(Dir2::Right),
        //CursorEdit::Insert(Dir2::Right, 25),
        //CursorEdit::Insert(Dir2::Left, 0),
        //CursorEdit::Insert(Dir2::Left, 0),
        //CursorEdit::Goto(Dir2::Right),
        //CursorEdit::Goto(Dir2::Right)
      ]
}

fn testcase5 () -> Edits {
    vec![CursorEdit::Insert(Dir2::Left, 5),
         CursorEdit::Insert(Dir2::Right, 5),
         CursorEdit::Goto(Dir2::Right),
         CursorEdit::Insert(Dir2::Left, 25),
         CursorEdit::Goto(Dir2::Left),
         CursorEdit::Insert(Dir2::Right, 37),
         CursorEdit::Replace(Dir2::Left, 1),
         CursorEdit::Goto(Dir2::Right),
         CursorEdit::Remove(Dir2::Right),
         CursorEdit::Replace(Dir2::Left, 20),
         CursorEdit::Goto(Dir2::Left),
         CursorEdit::Replace(Dir2::Right, 4),
         CursorEdit::Goto(Dir2::Left),
         CursorEdit::Goto(Dir2::Right),
         CursorEdit::Insert(Dir2::Right, 25),
         CursorEdit::Remove(Dir2::Left),
         CursorEdit::Insert(Dir2::Right, 5),
         CursorEdit::Replace(Dir2::Left, 11),
         CursorEdit::Insert(Dir2::Left, 30),
         CursorEdit::Goto(Dir2::Left),
         CursorEdit::Insert(Dir2::Left, 1),
         CursorEdit::Goto(Dir2::Right),
         CursorEdit::Goto(Dir2::Left),
         CursorEdit::Insert(Dir2::Right, 3),
         CursorEdit::Goto(Dir2::Left),
         CursorEdit::Insert(Dir2::Right, 16),
         CursorEdit::Insert(Dir2::Right, 31),
         CursorEdit::Insert(Dir2::Left, 24),
         CursorEdit::Goto(Dir2::Right),
         CursorEdit::Insert(Dir2::Left, 22),
         CursorEdit::Insert(Dir2::Left, 33),
         CursorEdit::Goto(Dir2::Right),
         CursorEdit::Insert(Dir2::Left, 3),
         CursorEdit::Insert(Dir2::Left, 1),
         CursorEdit::Goto(Dir2::Left),
         CursorEdit::Insert(Dir2::Right, 17),
         CursorEdit::Goto(Dir2::Left),
         CursorEdit::Insert(Dir2::Left, 34),
         CursorEdit::Replace(Dir2::Right, 9)]
}

// ---- ensure_consistency stdout ----
//     after edit 29: Replace(Dir2::Dir2::Right, 47): expected Max to be [47], but found [45]
//     thread 'ensure_consistency' panicked at '[Goto(Dir2::Dir2::Right), Insert(Dir2::Dir2::Left, 45), Remove(Dir2::Dir2::Right), Remove(Dir2::Dir2::Right), Insert(Dir2::Dir2::Right, 19), Goto(Dir2::Dir2::Left), Goto(Dir2::Dir2::Right), Insert(Dir2::Dir2::Left, 23), Goto(Dir2::Dir2::Right), Insert(Dir2::Dir2::Right, 14), Insert(Dir2::Dir2::Left, 28), Goto(Dir2::Dir2::Right), Goto(Dir2::Dir2::Right), Insert(Dir2::Dir2::Right, 37), Replace(Dir2::Dir2::Left, 35), Insert(Dir2::Dir2::Right, 0), Insert(Dir2::Dir2::Right, 23), Insert(Dir2::Dir2::Left, 27), Goto(Dir2::Dir2::Left), Insert(Dir2::Dir2::Right, 11), Insert(Dir2::Dir2::Right, 30), Insert(Dir2::Dir2::Right, 25), Insert(Dir2::Dir2::Right, 38), Goto(Dir2::Dir2::Right), Insert(Dir2::Dir2::Left, 24), Insert(Dir2::Dir2::Left, 45), Replace(Dir2::Dir2::Left, 31), Replace(Dir2::Dir2::Right, 43), Replace(Dir2::Dir2::Right, 2), Replace(Dir2::Dir2::Right, 47), Insert(Dir2::Dir2::Right, 46), Insert(Dir2::Dir2::Left, 27), Goto(Dir2::Dir2::Right), Remove(Dir2::Dir2::Right), Goto(Dir2::Dir2::Left), Insert(Dir2::Dir2::Left, 44), Goto(Dir2::Dir2::Left), Insert(Dir2::Dir2::Right, 27), Goto(Dir2::Dir2::Left)]', tests/listedit.rs:52

// ---- ensure_consistency stdout ----
//     after edit 41: Goto(Dir2::Dir2::Right): expected Max to be [49], but found [44]
//     thread 'ensure_consistency' panicked at '[Replace(Dir2::Dir2::Right, 45), Insert(Dir2::Dir2::Right, 0), Insert(Dir2::Dir2::Left, 12), Insert(Dir2::Dir2::Right, 22), Goto(Dir2::Dir2::Right), Goto(Dir2::Dir2::Right), Insert(Dir2::Dir2::Left, 16), Replace(Dir2::Dir2::Right, 7), Goto(Dir2::Dir2::Right), Insert(Dir2::Dir2::Left, 16), Insert(Dir2::Dir2::Right, 27), Insert(Dir2::Dir2::Left, 0), Goto(Dir2::Dir2::Right), Insert(Dir2::Dir2::Right, 36), Goto(Dir2::Dir2::Right), Goto(Dir2::Dir2::Right), Insert(Dir2::Dir2::Right, 11), Goto(Dir2::Dir2::Right), Insert(Dir2::Dir2::Right, 32), Remove(Dir2::Dir2::Left), Insert(Dir2::Dir2::Right, 14), Remove(Dir2::Dir2::Right), Goto(Dir2::Dir2::Right), Replace(Dir2::Dir2::Right, 23), Insert(Dir2::Dir2::Left, 34), Replace(Dir2::Dir2::Right, 49), Insert(Dir2::Dir2::Left, 0), Insert(Dir2::Dir2::Left, 16), Remove(Dir2::Dir2::Left), Insert(Dir2::Dir2::Left, 4), Goto(Dir2::Dir2::Right), Replace(Dir2::Dir2::Right, 23), Insert(Dir2::Dir2::Left, 44), Goto(Dir2::Dir2::Left), Insert(Dir2::Dir2::Left, 23), Goto(Dir2::Dir2::Right), Insert(Dir2::Dir2::Left, 18), Insert(Dir2::Dir2::Right, 15), Replace(Dir2::Dir2::Left, 49), Insert(Dir2::Dir2::Right, 16), Goto(Dir2::Dir2::Left), Goto(Dir2::Dir2::Right), Insert(Dir2::Dir2::Left, 46), Goto(Dir2::Dir2::Left)]', tests/listedit.rs:52

