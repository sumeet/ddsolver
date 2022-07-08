use lazy_static::lazy_static;
use std::mem::MaybeUninit;
use tinyvec::ArrayVec;

lazy_static! {
    static ref PERMS: Vec<Vec<[bool; 8]>> = {
        (0..=8)
            .map(|n| generate_perms(n).map(|perm| get_values(perm)).collect())
            .collect()
    };
}

enum Space {
    Empty,
    Wall,
    Monster,
    Treasure,
}

type Board = [[Option<Space>; 8]; 8];

fn generate_perms(constraint: u32) -> impl Iterator<Item = u8> {
    (0..=u8::MAX).filter(move |i| i.count_ones() == constraint)
}

fn get_values(walls_bin: u8) -> [bool; 8] {
    let mut values = [false; 8];
    for i in 0..8 {
        values[i] = walls_bin & (1 << (7 - i)) != 0 as u8;
    }
    values
}

fn main() {
    // dbg!(&*PERMS);

    let col_constraints = [2u8, 4, 4, 3, 2, 3, 4, 2];
    let row_constraints = [0u8, 7, 2, 4, 2, 2, 7, 0];

    let mut q = vec![ArrayVec::<[[bool; 8]; 8]>::new()];
    let mut found = vec![];
    while let Some(mut q_item) = q.pop() {
        if q_item.len() == 8 {
            found.push(q_item);
            continue;
        }

        for next_col in &PERMS[col_constraints[q_item.len()] as usize] {
            let mut next_q_item = q_item;
            next_q_item.push(*next_col);

            if row_constraints
                .into_iter()
                .enumerate()
                .all(|(row_index, constraint)| {
                    next_q_item
                        .iter()
                        .map(|col| col[row_index] as u8)
                        .sum::<u8>()
                        <= constraint
                })
            {
                q.push(next_q_item);
            }
        }
    }
    dbg!(found.len());
}

fn print_grid(cols: ArrayVec<[[bool; 8]; 8]>) {
    for x in 0..8 {
        for y in 0..8 {
            print!(
                "{}",
                if cols.get(y).and_then(|g| g.get(x).copied()) == Some(true) {
                    "#"
                } else {
                    "."
                }
            );
        }
        println!();
    }
}

// SCRATCH:
// use std::collections::HashSet;
// struct GuessingBoard {
//     constraints: HashSet<Constraint>,
// }
//
// type SpaceCoord = (u8, u8);
//
// enum SpaceConstraint {}
//
