#![feature(bool_to_option)]

use lazy_static::lazy_static;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use std::collections::HashSet;
use std::fs::read_to_string;
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

fn empty_board() -> Board {
    let mut b = Board::new();
    for _ in 0..8 {
        b.push([false; 8]);
    }
    b
}

fn main() {
    let s = read_to_string("./6.dd").unwrap();
    let b = ParsedBoard::parse(&s);

    println!("loaded grid:");
    reprint_grid(&empty_board(), &b);
    println!();
    println!("solving...");

    let col_constraints = b.col_constraints;
    let row_constraints = b.row_constraints;

    let mut q = vec![Board::new()];
    let mut found = vec![];
    while !q.is_empty() {
        (q, found) = q
            .par_iter()
            .fold_with((vec![], found), |(mut q, mut found), &q_item| {
                if q_item.len() == 8 {
                    found.push(q_item);
                    return (q, found);
                }

                let x = q_item.len();
                for next_col in &PERMS[col_constraints[x] as usize] {
                    let mut next_q_item = q_item;

                    for (y, &cell) in next_col.iter().enumerate() {
                        // we can't put a wall where there's a treasure or monster
                        if cell && (b.treasure_locations[x][y] || b.monster_locations[x][y]) {
                            continue;
                        }
                    }
                    next_q_item.push(*next_col);

                    if row_constraints
                        .into_iter()
                        .enumerate()
                        .all(|(y, constraint)| {
                            next_q_item.iter().map(|col| col[y] as u8).sum::<u8>() <= constraint
                        })
                    {
                        q.push(next_q_item);
                    }
                }

                (q, found)
            })
            .reduce(
                || (vec![], vec![]),
                |(mut q_a, mut found_a), (q_b, found_b)| {
                    q_a.extend(q_b);
                    found_a.extend(found_b);
                    (q_a, found_a)
                },
            );
    }
    // while let Some(q_item) = q.pop() {
    //     if q_item.len() == 8 {
    //         found.push(q_item);
    //         continue;
    //     }
    //
    //     let x = q_item.len();
    //     for next_col in &PERMS[col_constraints[x] as usize] {
    //         let mut next_q_item = q_item;
    //
    //         for (y, &cell) in next_col.iter().enumerate() {
    //             // we can't put a wall where there's a treasure or monster
    //             if cell && (b.treasure_locations[x][y] || b.monster_locations[x][y]) {
    //                 continue;
    //             }
    //         }
    //
    //         next_q_item.push(*next_col);
    //
    //         if row_constraints
    //             .into_iter()
    //             .enumerate()
    //             .all(|(y, constraint)| {
    //                 next_q_item.iter().map(|col| col[y] as u8).sum::<u8>() <= constraint
    //             })
    //         {
    //             q.push(next_q_item);
    //         }
    //     }
    // }
    println!("- after filtering col and row constraints");
    dbg!(found.len());

    // filter out non-contiguous grids
    let (contiguous, _noncontiguous): (Vec<Board>, Vec<Board>) =
        found.into_par_iter().partition(|grid| is_contiguous(*grid));
    println!("- after filtering out non-contiguous grids");
    dbg!(contiguous.len());

    // keep only grids with monsters in dead ends
    let with_monsters_in_dead_ends = contiguous
        .into_iter()
        .filter(|board| {
            b.all_monster_positions().all(|monster_pos| {
                let nbors = neighbors(monster_pos);
                let num_nbors_are_spaces = nbors.filter(|(x, y)| !board[*x][*y]).count();
                num_nbors_are_spaces == 1
            })
        })
        .collect::<Vec<_>>();
    println!("- after filtering out grids with monsters not in dead ends");
    dbg!(with_monsters_in_dead_ends.len());

    // keep only grids with all dead ends containing monsters
    let with_all_dead_ends_containing_monsters = with_monsters_in_dead_ends
        .into_iter()
        .filter(|board| {
            // filter only dead ends
            let mut dead_ends = board.iter().enumerate().flat_map(|(x, col)| {
                col.into_iter().enumerate().flat_map(move |(y, &cell)| {
                    if !cell {
                        let nbors = neighbors((x, y));
                        let num_nbors_are_spaces = nbors.filter(|(x, y)| !board[*x][*y]).count();
                        if num_nbors_are_spaces == 1 {
                            Some((x, y))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
            });
            dead_ends.all(|(x, y)| b.monster_locations[x][y])
        })
        .collect::<Vec<_>>();
    println!("- after filtering out grids with dead ends not containing monsters");
    dbg!(with_all_dead_ends_containing_monsters.len());

    for soln in with_all_dead_ends_containing_monsters {
        reprint_grid(&soln, &b);
        println!();
    }
}

fn reprint_grid(grid: &Board, b: &ParsedBoard) {
    print!(" ");
    for cc in b.col_constraints {
        print!("{}", cc);
    }
    println!();
    for y in 0..8 {
        print!("{}", b.row_constraints[y]);
        for x in 0..8 {
            if b.monster_locations[x][y] {
                print!("m");
            } else if b.treasure_locations[x][y] {
                print!("t");
            } else {
                if grid[x][y] {
                    print!("#");
                } else {
                    print!(".");
                }
            }
        }
        println!();
    }
}

#[derive(Debug)]
struct ParsedBoard {
    col_constraints: [u8; 8],
    row_constraints: [u8; 8],
    monster_locations: [[bool; 8]; 8],
    treasure_locations: [[bool; 8]; 8],
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
        let mut treasure_locations = [[false; 8]; 8];
        for (y, line) in lines.enumerate() {
            let mut chars = line.trim().chars();
            let first_char = chars.next().unwrap();
            row_constraints[y] = first_char.to_digit(10).unwrap() as _;
            for (x, rest) in chars.enumerate() {
                if rest == 'm' {
                    monster_locations[x][y] = true;
                } else if rest == 't' {
                    treasure_locations[x][y] = true;
                }
            }
        }
        Self {
            col_constraints,
            row_constraints,
            monster_locations,
            treasure_locations,
        }
    }

    fn all_monster_positions(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.monster_locations
            .iter()
            .enumerate()
            .flat_map(|(x, col)| {
                col.iter()
                    .enumerate()
                    .filter_map(move |(y, &cell)| cell.then_some((x, y)))
            })
    }
}

enum Space {
    Empty,
    Wall,
    Monster,
    Treasure,
}

type Board = ArrayVec<[[bool; 8]; 8]>;

fn neighbors((x, y): (usize, usize)) -> impl Iterator<Item = (usize, usize)> {
    let (x, y) = (x as isize, y as isize);
    [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
        .into_iter()
        .filter(|&(x, y)| x >= 0 && y >= 0 && x < 8 && y < 8)
        .map(|(x, y)| (x as _, y as _))
}

fn is_contiguous(b: Board) -> bool {
    let all_space_coordinates = b
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
        for (nx, ny) in neighbors((x, y)) {
            if all_space_coordinates.contains(&(nx, ny)) && !visited.contains(&(nx, ny)) {
                to_visit.push((nx, ny));
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
