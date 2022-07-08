use lazy_static::lazy_static;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::collections::HashSet;
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

type Board = ArrayVec<[[bool; 8]; 8]>;

fn is_contiguous(b: Board) -> bool {
    let mut all_space_coordinates = b
        .iter()
        .enumerate()
        .flat_map(|(x, col)| {
            col.iter()
                .enumerate()
                .map(move |(y, &cell)| if !cell { Some((x, y)) } else { None })
        })
        .flatten()
        .collect::<HashSet<(usize, usize)>>();

    // start at one space, and see if we can get to the rest of them. if so, then we got it
    let mut found_spaces = HashSet::new();
    let mut visited = HashSet::new();
    let mut to_visit = vec![all_space_coordinates.iter().copied().next().unwrap()];
    while let Some((x, y)) = to_visit.pop() {
        visited.insert((x, y));

        if all_space_coordinates.contains(&(x, y)) {
            found_spaces.insert((x, y));
        }
        for (dx, dy) in &[(1isize, 0isize), (-1, 0), (0, 1), (0, -1)] {
            let (nx, ny) = ((x as isize + dx) as isize, (y as isize + dy) as isize);
            if nx >= 0
                && nx < 8
                && ny >= 0
                && ny < 8
                && all_space_coordinates.contains(&(nx as _, ny as _))
            {
                if !visited.contains(&(nx as _, ny as _)) {
                    to_visit.push((nx as _, ny as _));
                }
            }
        }
    }

    all_space_coordinates.intersection(&found_spaces).count() == all_space_coordinates.len()
}

fn main() {
    // dbg!(&*PERMS);

    let col_constraints = [2u8, 4, 4, 3, 2, 3, 4, 2];
    let row_constraints = [0u8, 7, 2, 4, 2, 2, 7, 0];

    let mut q = vec![Board::new()];
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
    println!("- after filtering col and row constraints");
    dbg!(found.len());

    // filter out non-contiguous grids
    let (contiguous, noncontiguous): (Vec<Board>, Vec<Board>) =
        found.into_par_iter().partition(|grid| is_contiguous(*grid));
    // for found in found {
    //     print_grid(found);
    //     println!();
    // }
    println!("- after filtering out non-contiguous grids");
    dbg!(contiguous.len());
    dbg!(noncontiguous.len());
    println!("example of contiguous grid:");
    print_grid(contiguous[0]);
    println!();
    println!("example of non-contiguous grid:");
    print_grid(noncontiguous[0]);
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
