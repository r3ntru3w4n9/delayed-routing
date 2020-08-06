use crate::utilities::UnionFind;
use anyhow::{Error, Result};
use arrayvec::ArrayVec;
use num::Num;
use std::{
    cmp,
    collections::{HashMap, HashSet},
    fmt::{Display, Error as FmtError, Formatter, Result as FmtResult},
    str::FromStr,
    usize,
};

/// FactoryID provides three methods.
/// Users only need to implement `prefix`
/// which is a `&'static str` and unique to the type.
/// `from_str` generates an instance's id,
/// `from_numeric` generates an instance's name.
/// Both are automatically implemented provided that `prefix` is implemented.
pub trait FactoryID {
    /// The prefix a type has in input file.
    /// The name of an instance is uniquely determined by its prefix and id
    fn prefix() -> &'static str;

    /// Converts from &str to usize.
    fn from_str(name: &str) -> Result<usize>
    where
        Error: From<<usize as FromStr>::Err>,
    {
        // subtracted by one because of the offset
        let length = Self::prefix().len();

        Ok(name[length..].parse::<usize>().map_err(Error::from)? - 1)
    }

    /// Converts from usize to String.
    fn from_numeric(id: usize) -> Result<String> {
        // added by one because of the offset
        Ok(format!("{}{}", Self::prefix(), id + 1))
    }
}

/// Directions of a layer
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Direction {
    Horizontal,
    Vertical,
}

/// There are different conflict types
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ConflictType {
    AdjHGGrid,
    SameGGrid,
}

/// Whether a cell is movable
#[derive(Debug)]
pub enum CellType {
    Movable,
    Fixed,
}

/// Towards a direction
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Towards {
    Up,
    Down,
    Left,
    Right,
    Top,
    Bottom,
}

/// A 2-dimension tuple representing a Pair.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Pair<T>(pub T, pub T)
where
    T: Copy + Num;

/// A 3-dimension tuple representing a Point.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Point<T>(pub T, pub T, pub T)
where
    T: Copy + Num;

/// A source point and a target point representing a Route.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Route<T>(pub Point<T>, pub Point<T>)
where
    T: Copy + Num;

/// Some information about a Layer.
#[derive(Debug)]
pub struct Layer {
    /// layer id (starts from 0)
    pub id: usize,
    /// horizontal or vertical
    pub direction: Direction,
    /// dimensions
    pub dim: Pair<usize>,
    /// all grids' capacity
    pub capacity: Vec<usize>,
}

/// Some information about a MasterPin.

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct MasterPin {
    /// id of the pin
    pub id: usize,
    /// layer on which the pin is on
    pub layer: usize,
}

/// Some information about a Blockage.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Blockage {
    /// id of the blockage
    pub id: usize,
    /// layer on which the blockage is on
    pub layer: usize,
    /// extra demand the blockage will cost
    pub demand: usize,
}

/// Some information about a MasterCell.
#[derive(Debug)]
pub struct MasterCell {
    /// id of cell
    pub id: usize,
    /// number of pins
    pub pins: HashSet<MasterPin>,
    /// number of blockages
    pub blkgs: HashSet<Blockage>,
}

/// Some information about a Conflict,
/// which happens when certain types of MasterCells are too close for confort.
#[derive(Debug, Eq, Hash, PartialEq)]
pub struct Conflict {
    /// adjHGGrid or sameGGrid
    pub kind: ConflictType,
    /// other id
    pub id: usize,
    /// on which layer
    pub layer: usize,
    /// by how much
    pub demand: usize,
}

/// Some information about a Cell
#[derive(Debug)]
pub struct Cell {
    /// id of the cell
    pub id: usize,
    /// if the cell can be moved
    pub movable: CellType,
    /// position
    pub position: Pair<usize>,
    /// mastercell type
    pub pins: Vec<usize>,
}

/// Pointer points to the nearby node.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Pointer {
    /// nearby node index
    index: usize,
    /// nearby node height
    height: usize,
}

/// A node in a tree.
#[derive(Clone, Copy, Debug)]
pub struct NetNode {
    /// corresponding to pin id, None represents a virtual node.
    pub id: Option<usize>,
    /// positions
    pub position: Pair<usize>,
    /// nearby nodes
    pub up: Option<Pointer>,
    /// nearby nodes
    pub down: Option<Pointer>,
    /// nearby nodes
    pub left: Option<Pointer>,
    /// nearby nodes
    pub right: Option<Pointer>,
}

/// Net represented as a tree.
#[derive(Debug)]
pub struct NetTree {
    /// All nodes in a tree
    nodes: Vec<NetNode>,
}

/// Some information about a Net.
#[derive(Debug)]
pub struct Net {
    /// id of the net
    pub id: usize,
    /// min layer id
    pub min_layer: usize,
    /// Structure of the net represented as a tree
    pub tree: NetTree,
}

impl<T> Pair<T>
where
    T: Copy + Num,
{
    pub fn x(&self) -> T {
        self.0
    }

    pub fn y(&self) -> T {
        self.1
    }

    pub fn size(&self) -> T {
        // x: rows, y: columns
        self.x() * self.y()
    }

    pub fn with(self, lay: T) -> Point<T> {
        let Pair(row, col) = self;
        Point(row, col, lay)
    }
}

impl<T> Point<T>
where
    T: Copy + Num,
{
    pub fn row(&self) -> T {
        self.0
    }

    pub fn col(&self) -> T {
        self.1
    }

    pub fn lay(&self) -> T {
        self.2
    }

    pub fn flatten(&self) -> Pair<T> {
        let &Point(row, col, _) = self;
        Pair(row, col)
    }
}

impl<T> Route<T>
where
    T: Copy + Num,
{
    pub fn source(&self) -> Point<T> {
        self.0
    }

    pub fn target(&self) -> Point<T> {
        self.1
    }
}

impl Layer {
    pub fn get_capacity(&self, row: usize, col: usize) -> Option<&usize> {
        self.capacity.get(row * self.dim.y() + col)
    }

    pub fn get_capacity_mut(&mut self, row: usize, col: usize) -> Option<&mut usize> {
        self.capacity.get_mut(row * self.dim.y() + col)
    }
}

impl FactoryID for Layer {
    fn prefix() -> &'static str {
        "M"
    }
}

impl FactoryID for MasterPin {
    fn prefix() -> &'static str {
        "P"
    }
}

impl FactoryID for Blockage {
    fn prefix() -> &'static str {
        "B"
    }
}

impl FactoryID for MasterCell {
    fn prefix() -> &'static str {
        "MC"
    }
}

impl FactoryID for Cell {
    fn prefix() -> &'static str {
        "C"
    }
}

impl<T> Display for Point<T>
where
    T: Copy + Display + Num,
{
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{} {} {}", self.row(), self.col(), self.lay())
    }
}

impl Route<usize> {
    fn vector(&self) -> Point<isize> {
        let Route(source, target) = self;
        Point(
            target.row() as isize - source.row() as isize,
            target.col() as isize - source.col() as isize,
            target.lay() as isize - source.lay() as isize,
        )
    }

    pub fn towards(&self) -> Towards {
        match self.vector() {
            Point(0, 0, 0) => unreachable!(),
            Point(row, 0, 0) => {
                if row > 0 {
                    Towards::Up
                } else {
                    Towards::Down
                }
            }
            Point(0, col, 0) => {
                if col > 0 {
                    Towards::Right
                } else {
                    Towards::Left
                }
            }
            Point(0, 0, lay) => {
                if lay > 0 {
                    Towards::Top
                } else {
                    Towards::Bottom
                }
            }
            _ => unreachable!(),
        }
    }
}

impl<T> Display for Route<T>
where
    T: Copy + Display + Num,
{
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{} {}", self.source(), self.target())
    }
}

impl Towards {
    pub fn inv(&self) -> Self {
        match self {
            Towards::Up => Towards::Down,
            Towards::Down => Towards::Up,
            Towards::Left => Towards::Right,
            Towards::Right => Towards::Left,
            Towards::Top => Towards::Bottom,
            Towards::Bottom => Towards::Top,
        }
    }
}

impl NetNode {
    pub fn neightbors(&self) -> [Option<Pointer>; 4] {
        [self.up, self.down, self.left, self.right]
    }

    pub fn span(&self) -> (usize, usize) {
        self.neightbors()
            .iter()
            .filter_map(|opt| *opt)
            .map(|ptr| ptr.height)
            .fold((usize::MAX, usize::MIN), |(min, max), height| {
                (cmp::min(min, height), cmp::max(max, height))
            })
    }

    pub fn index(self, towards: Towards) -> Option<Pointer> {
        match towards {
            Towards::Up => self.up,
            Towards::Down => self.down,
            Towards::Left => self.left,
            Towards::Right => self.right,
            Towards::Top | Towards::Bottom => unreachable!(),
        }
    }
}

impl NetTree {
    pub fn new<F>(conn_pins: Vec<usize>, segments: HashSet<Route<usize>>, pin_position: F) -> Self
    where
        F: Fn(usize) -> Option<Pair<usize>>,
    {
        // Using handcrafted `fold` first instead of direct using `collect` here
        // to bypass implementation details of `collect`
        let mut nodes: Vec<NetNode> = segments
            .iter()
            .map(|&Route(source, target)| [source, target])
            .map(ArrayVec::from)
            .map(ArrayVec::into_iter)
            .flatten()
            .map(|ref pt| pt.flatten())
            .map(|pin| (pin, None))
            .chain(conn_pins.into_iter().map(|idx| {
                (
                    pin_position(idx).expect("Pin not found in database"),
                    Some(idx),
                )
            }))
            .fold(HashMap::new(), |mut hmap, (position, idx)| {
                *hmap.entry(position).or_insert(Option::default()) = idx;
                hmap
            })
            .into_iter()
            .map(|(position, id)| NetNode {
                id,
                position,
                up: None,
                down: None,
                left: None,
                right: None,
            })
            .collect();

        let num_nodes = nodes.len();

        let position_to_idx: HashMap<Pair<usize>, usize> = nodes
            .iter()
            .enumerate()
            .map(|(idx, node)| (node.position, idx))
            .collect();

        debug_assert_eq!(
            nodes
                .iter()
                .map(|node| { node.position })
                .collect::<HashSet<_>>()
                .len(),
            num_nodes
        );

        debug_assert_eq!(position_to_idx.len(), num_nodes);

        let mut union_find = UnionFind::new(num_nodes);

        let mut uf_cnt = 0;

        for route in segments.into_iter().filter(|elem| match elem.towards() {
            Towards::Up | Towards::Down | Towards::Left | Towards::Right => true,
            Towards::Top | Towards::Bottom => false,
        }) {
            let towards = route.towards();
            let Route(source, target) = route;

            let height = source.lay();
            debug_assert_eq!(height, target.lay());
            match towards {
                Towards::Up | Towards::Down | Towards::Left | Towards::Right => (),
                Towards::Top | Towards::Bottom => unreachable!(),
            }

            let source_pos = source.flatten();
            let target_pos = target.flatten();

            let source_idx = *position_to_idx
                .get(&source_pos)
                .expect("Index out of bounds");
            let target_idx = *position_to_idx
                .get(&target_pos)
                .expect("Index out of bounds");

            if !union_find.union(source_idx, target_idx) {
                debug_assert_eq!(uf_cnt, 0);
                uf_cnt += 1;
                continue;
            }

            Self::connect(&mut nodes, source_idx, target_idx, height, towards);
        }

        debug_assert!(union_find.done());
        debug_assert_eq!(uf_cnt, 1);

        Self { nodes }
    }

    /// Connects two different nodes.
    fn connect(nodes: &mut [NetNode], sindex: usize, oindex: usize, height: usize, diff: Towards) {
        let mut set_node = |sindex: usize, oindex: usize, diff: Towards| {
            let node: &mut NetNode = nodes.get_mut(sindex).expect("Node does not exist");
            let some_ptr = Some(Pointer {
                index: oindex,
                height,
            });

            match diff {
                Towards::Up => node.up = some_ptr,
                Towards::Down => node.down = some_ptr,
                Towards::Left => node.left = some_ptr,
                Towards::Right => node.right = some_ptr,
                Towards::Top | Towards::Bottom => unreachable!(),
            }
        };

        debug_assert_ne!(sindex, oindex);

        set_node(sindex, oindex, diff);
        set_node(oindex, sindex, diff.inv());
    }
}

impl Net {
    pub fn new<F>(
        id: usize,
        min_layer: usize,
        conn_pins: Vec<usize>,
        segments: HashSet<Route<usize>>,
        pin_position: F,
    ) -> Self
    where
        F: Fn(usize) -> Option<Pair<usize>>,
    {
        let tree = NetTree::new(conn_pins, segments, pin_position);
        Self {
            id,
            min_layer,
            tree,
        }
    }

    fn fmt_recursive(
        &self,
        f: &mut Formatter,
        node: NetNode,
        list: &[NetNode],
        name: &str,
        direction: Towards,
    ) -> FmtResult {
        let directions =
            ArrayVec::from([Towards::Up, Towards::Down, Towards::Left, Towards::Right]);

        for dir in directions.into_iter() {
            if dir == direction.inv() {
                continue;
            }
            let Pointer { index, height } = match node.index(dir) {
                Some(idx) => idx,
                None => continue,
            };
            let nearby_node = *list.get(index).expect("Index out of bounds");

            let source = node.position.with(height);
            let target = nearby_node.position.with(height);

            write!(f, "{} {}\n", Route(source, target), name)?;

            self.fmt_recursive(f, nearby_node, list, name, dir)?;
        }

        Ok(())
    }
}

impl Display for Net {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        let name = &Self::from_numeric(self.id).map_err(|_| FmtError)?;

        for node in self.tree.nodes.iter() {
            let Pair(row, col) = node.position;
            let (min, max) = node.span();
            write!(f, "{} {} {} ", row, col, min)?;
            write!(f, "{} {} {} ", row, col, max)?;
            write!(f, "{}\n", name)?;
        }

        let directions =
            ArrayVec::from([Towards::Up, Towards::Down, Towards::Left, Towards::Right]);
        if let Some(&root) = self.tree.nodes.first() {
            for dir in directions.into_iter() {
                self.fmt_recursive(f, root, &self.tree.nodes, name, dir)?;
            }
        }

        Ok(())
    }
}

impl FactoryID for Net {
    fn prefix() -> &'static str {
        "N"
    }
}
