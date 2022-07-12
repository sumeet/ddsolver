use bitvec::macros::internal::funty::Fundamental;
use bitvec::order::{Lsb0, Msb0};
use bitvec::prelude::BitStore;
use bitvec::view::BitView;
use lazy_static::lazy_static;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::fs::read_to_string;
use tinyvec::{array_vec, ArrayVec};

lazy_static! {
    // // this could be a constant / lazy_static, also unneeded right now
    // let mut col_masks = [0u64; 8];
    // for (x, col_mask) in col_masks.iter_mut().enumerate() {
    //     let col_mask_bits = col_mask.view_bits_mut::<Lsb0>();
    //     for i in (x * 8)..((x * 8) + 8) {
    //         col_mask_bits.set(i, true);
    //     }
    // }
    static ref ROW_MASKS : [u64; 8] = {
       let mut row_masks =  [0u64; 8];
        for (y, row_mask) in row_masks
            .iter_mut()
            .enumerate()
        {
            // HMM weird, not sure why we need Msb0 here??
            let row_mask_bits = row_mask.view_bits_mut::<Msb0>();
            for x in 0..8 {
                row_mask_bits.set(y + (x * 8), true);
            }
        }
        row_masks
    };


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
    let s = read_to_string("./5.dd").unwrap();
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
        println!("q.length: {}", q.len());
        (q, found) = q
            .par_iter()
            .fold_with((vec![], found), |(mut q, mut found), &q_item| {
                if q_item.len() == 8 {
                    found.push(q_item);
                    return (q, found);
                }

                let x = q_item.len();
                'perm: for next_col in &PERMS[col_constraints[x] as usize] {
                    let mut next_q_item = q_item;

                    for (y, &cell) in next_col.iter().enumerate() {
                        // we can't put a wall where there's a treasure or monster
                        if cell && (b.treasure_locations[x][y] || b.monster_locations[x][y]) {
                            continue;
                        }
                    }
                    next_q_item.push(*next_col);

                    let mut board_bits = 0u64;
                    let mut any_open_cell = None;
                    let board_bits_view = board_bits.view_bits_mut::<Lsb0>();

                    for (x, col) in next_q_item.iter().enumerate() {
                        for (y, &cell) in col.iter().enumerate() {
                            if cell {
                                board_bits_view.set(x * 8 + y, true);
                            } else {
                                any_open_cell = Some(x * 8 + y);
                            }
                        }
                    }

                    // check row constraints (we don't need to check col constraints because we
                    // generate PERMS using those to begin with)
                    let mut new_method = true;
                    if ROW_MASKS.into_iter().zip(row_constraints).any(
                        |(row_mask, row_constraint)| {
                            dbg!(row_mask.to_le_bytes().map(|b| format!("{:08b}", b)));
                            dbg!(board_bits.to_le_bytes().map(|b| format!("{:08b}", b)));
                            (row_mask & board_bits).count_ones() as u8 > dbg!(row_constraint)
                        },
                    ) {
                        new_method = false;
                    }
                    // 'new_method: for (x, row_mask) in (&ROW_MASKS).iter().enumerate() {
                    //     if (row_mask & board_bits).count_ones() as u8 > row_constraints[x] {
                    //         new_method = false;
                    //         break 'new_method;
                    //         continue 'perm;
                    //     }
                    // }

                    // // old (slow?) method
                    let mut old_method = true;
                    dbg!(next_q_item);
                    if row_constraints
                        .into_iter()
                        .enumerate()
                        .any(|(y, constraint)| {
                            next_q_item.iter().map(|col| col[y] as u8).sum::<u8>() > constraint
                        })
                    {
                        new_method = false;
                        // continue 'perm;
                    }

                    if dbg!(new_method) != dbg!(old_method) {
                        panic!("mismatch");
                    }

                    if !new_method || !old_method {
                        continue 'perm;
                    }

                    // check contiguity last, i think it's the slowest
                    if !is_contiguous(board_bits, any_open_cell.unwrap()) {
                        continue;
                    }

                    q.push(next_q_item);
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
    println!("- after filtering col and row constraints");
    dbg!(found.len());

    //// filter out non-contiguous grids
    //let (contiguous, _noncontiguous): (Vec<Board>, Vec<Board>) =
    //    found.into_par_iter().partition(|grid| is_contiguous(*grid));
    //println!("- after filtering out non-contiguous grids");
    //dbg!(contiguous.len());

    // keep only grids with monsters in dead ends
    let with_monsters_in_dead_ends = found
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
                    // a cell with a wall can't be a dead end
                    if cell {
                        return None;
                    }

                    // if a cell is empty, then it is a dead end if it has exactly one empty neighbor
                    let nbors = neighbors((x, y));
                    let num_nbors_are_spaces = nbors.filter(|(x, y)| !board[*x][*y]).count();
                    if num_nbors_are_spaces == 1 {
                        return Some((x, y));
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

// TODO: maybe this should just be a u64
type Board = ArrayVec<[[bool; 8]; 8]>;

fn neighbors((x, y): (usize, usize)) -> impl Iterator<Item = (usize, usize)> {
    let (x, y) = (x as isize, y as isize);
    [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
        .into_iter()
        .filter(|&(x, y)| x >= 0 && y >= 0 && x < 8 && y < 8)
        .map(|(x, y)| (x as _, y as _))
}

fn is_contiguous(board_bits: u64, any_open_cell: usize) -> bool {
    // TODO: we're constructing this twice, this might be slowing things down
    let board_bits_view = board_bits.view_bits::<Lsb0>();

    // start at one space, and see if we can get to the rest of them. if so, then we got it
    let mut found_spaces_bits = 0u64;
    let mut found_spaces_bits_view = found_spaces_bits.view_bits_mut::<Lsb0>();
    let mut visited_bits = 0u64;
    let mut visited_bits_view = visited_bits.view_bits_mut::<Lsb0>();
    let mut to_visit = array_vec![[usize; 64] => any_open_cell];
    while let Some(i) = to_visit.pop() {
        visited_bits_view.set(i, true);

        if !board_bits_view[i] {
            found_spaces_bits_view.set(i, true);
        }
        let (x, y) = (i / 8, i % 8);
        for (nx, ny) in neighbors((x, y)) {
            let ni = nx * 8 + ny;
            if !board_bits_view[ni] && !visited_bits_view[ni] {
                to_visit.push(ni);
            }
        }
    }
    !found_spaces_bits == board_bits
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
