use lazy_static::lazy_static;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::collections::HashSet;
use std::fs::read_to_string;
use std::mem::MaybeUninit;
use tinyvec::ArrayVec;

lazy_static! {
    static ref PERMS: Vec<Vec<[bool; 8]>> = {
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

        (0..=8)
            .map(|n| generate_perms(n).map(|perm| get_values(perm)).collect())
            .collect()
    };
}

fn main() {
    let s = read_to_string("./5.dd").unwrap();
    let b = ParsedBoard::parse(&s);
    let col_constraints = b.col_constraints;
    let row_constraints = b.row_constraints;

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
    let (contiguous, _noncontiguous): (Vec<Board>, Vec<Board>) =
        found.into_par_iter().partition(|grid| is_contiguous(*grid));
    println!("- after filtering out non-contiguous grids");
    dbg!(contiguous.len());

    // filter out grids that contain monsters where there are walls
    let (monsters_filtered_out, monsters_overlapping): (Vec<_>, Vec<_>) =
        contiguous.into_iter().partition(|board| {
            board.iter().enumerate().all(|(x, col)| {
                col.into_iter().enumerate().all(|(y, &cell)| {
                    if cell {
                        !b.monster_locations[x][y]
                    } else {
                        true
                    }
                })
            })
        });
    println!("- after filtering out grids with walls overlapping monster positions");
    dbg!(monsters_filtered_out.len());
    println!("- example of monster not overlapping (GOOD)");
    print_grid(monsters_filtered_out[0]);
    println!("- example of monster overlapping (BAD)");
    print_grid(monsters_overlapping[0]);
}

#[derive(Debug)]
struct ParsedBoard {
    col_constraints: [u8; 8],
    row_constraints: [u8; 8],
    monster_locations: [[bool; 8]; 8],
}

impl ParsedBoard {
    fn parse(s: &str) -> Self {
        let s = s.trim();
        let mut lines = s.lines();
        let first_line = lines.next().unwrap().trim();
        let mut col_constraints = [0; 8];
        for (i, char) in first_line.chars().enumerate() {
            col_constraints[i] = char.to_digit(10).unwrap() as _;
        }
        let mut row_constraints = [0; 8];
        let mut monster_locations = [[false; 8]; 8];
        for (y, line) in lines.enumerate() {
            let mut chars = line.trim().chars();
            let first_char = chars.next().unwrap();
            row_constraints[y] = first_char.to_digit(10).unwrap() as _;
            for (x, rest) in chars.enumerate() {
                if rest == 'm' {
                    monster_locations[x][y] = true;
                }
            }
        }
        Self {
            col_constraints,
            row_constraints,
            monster_locations,
        }
    }
}

enum Space {
    Empty,
    Wall,
    Monster,
    Treasure,
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
