use serde::{Serialize, Deserialize};
use crate::result::*;
use std::collections::VecDeque;
use std::collections::BTreeMap;
use std::cmp::Ordering;
use std::fmt;

const POINT_PRECISION: f64 = 0.00001;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl std::hash::Hash for Point {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_i64((self.x / POINT_PRECISION) as i64);
        state.write_i64((self.y / POINT_PRECISION) as i64);
    }
}

impl PartialEq for Point {
    fn eq(&self, other: &Point) -> bool {
        (self.x - other.x).abs() < POINT_PRECISION &&
        (self.y - other.y).abs() < POINT_PRECISION
    }
}

impl Eq for Point {}

impl std::cmp::Ord for Point {
    fn cmp(&self, other: &Point) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for Point {
    fn partial_cmp(&self, other: &Point) -> Option<Ordering> {
        Some(self.x.partial_cmp(&other.x)?.then(self.y.partial_cmp(&other.y)?))
    }
}

#[allow(unused)]
impl Point {
    pub fn dist(&self, other: &Point) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }

    pub fn angle(&self, other: &Point) -> f64 {
        (self.y - other.y).atan2(self.x - other.x)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Entity {
    Line(Point, Point),
    Arc {
        center: Point,
        radius: f64,
        start_angle: f64,
        end_angle: f64,
    },
    Circle {
        center: Point,
        radius: f64,
    },
    Polyline {
        // curve_fit: bool,
        // spline_fit: bool,
        /*
        0 = No smooth surface fitted
        5 = Quadratic B-spline surface
        6 = Cubic B-spline surface
        8 = Bezier surface
        */
        curve_type: u32,
        vertices: Vec<Point>,
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Drawing {
    pub entities: Vec<Entity>,
}

fn eof() -> Error {
    weld_err!("unexpected eof")
}

fn unexpected_tag(expected: i32, got: i32) -> Error {
    weld_err!("unexpected tag: {}, expected: {}", got, expected)
}

fn missing_tag_for_entity(tag: i32) -> Error {
    weld_err!("missing tag for entity: {}", tag)
}

fn emit<T: fmt::Display>(out: &mut String, tag: i32, data: T) {
    out.push_str(&format!("  {}\n{}\n", tag, data));
}

impl ToString for Drawing {
    fn to_string(&self) -> String {
        let mut out = String::new();
        // emit(&mut out, 0, "SECTION");
        // emit(&mut out, 2, "HEADER");
        // emit(&mut out, 9, "$ACADVER");
        // emit(&mut out, 1, "AC1014");
        // emit(&mut out, 9, "$MEASUREMENT");
        // emit(&mut out, 70, 1);
        // emit(&mut out, 0, "ENDSEC");

        emit(&mut out, 0, "SECTION");
        emit(&mut out, 2, "BLOCKS");
        emit(&mut out, 0, "ENDSEC");

        emit(&mut out, 0, "SECTION");
        emit(&mut out, 2, "ENTITIES");
        for entity in self.entities.iter() {
            match entity {
                Entity::Line(left, right) => {
                    emit(&mut out, 0, "LINE");
                    emit(&mut out, 8, 0.0);
                    emit(&mut out, 10, left.x);
                    emit(&mut out, 20, left.y);
                    emit(&mut out, 11, right.x);
                    emit(&mut out, 21, right.y);
                },
                Entity::Arc { center, radius, start_angle, end_angle } => {
                    emit(&mut out, 0, "ARC");
                    emit(&mut out, 8, 0.0);
                    emit(&mut out, 10, center.x);
                    emit(&mut out, 20, center.y);
                    emit(&mut out, 40, *radius);
                    emit(&mut out, 50, *start_angle);
                    emit(&mut out, 51, *end_angle);
                },
                Entity::Circle { center, radius } => {
                    emit(&mut out, 0, "CIRCLE");
                    emit(&mut out, 8, 0.0);
                    emit(&mut out, 10, center.x);
                    emit(&mut out, 20, center.y);
                    emit(&mut out, 40, *radius);
                },
                Entity::Polyline { curve_type, vertices } => {
                    emit(&mut out, 0, "POLYLINE");
                    emit(&mut out, 8, 0.0);
                    emit(&mut out, 70, 8u32);
                    emit(&mut out, 75, curve_type);
                    for Point { x, y } in vertices.iter() {
                        emit(&mut out, 0, "VERTEX");
                        emit(&mut out, 8, 0.0);
                        emit(&mut out, 70, 32u32);
                        emit(&mut out, 10, x);
                        emit(&mut out, 20, y);
                    }
                    emit(&mut out, 0, "SEQEND");
                },
            }
        }
        emit(&mut out, 0, "ENDSEC");
        emit(&mut out, 0, "SECTION");
        emit(&mut out, 2, "OBJECTS");
        emit(&mut out, 0, "DICTIONARY");
        emit(&mut out, 0, "ENDSEC");
        emit(&mut out, 0, "EOF");

        out
    }
}

impl Drawing {

    pub fn parse(src: &str) -> Result<Drawing> {
        let mut lines = src.split('\n').map(|x| x.trim()).filter(|x| x.len() > 0).collect::<VecDeque<&str>>();
        let mut entities = vec![];
        let mut state = 0;
        let mut entity_type = "";
        let mut entity_state: BTreeMap<i32, &str> = BTreeMap::new();
        while lines.len() > 0 {
            let tag = lines.pop_front().unwrap().parse::<i32>()?;
            let value = lines.pop_front().ok_or_else(eof)?;
            if state == 0 {
                if value == "EOF" {
                    break;
                } else if value != "SECTION" {
                    return Err(weld_err!("expected SECTION, got {}", value));
                }
                state = 1;
                continue;
            } else if state == 1 {
                if tag == 2 {
                    if value == "ENTITIES" {
                        state = 3;
                    } else {
                        state = 2;
                    }
                } else {
                    return Err(unexpected_tag(2, tag));
                }
                continue;
            } else if state == 2 {
                if tag == 0 {
                    if value == "ENDSEC" {
                        state = 0;
                    }
                }
                continue;
            } else if state == 4 {
                if tag == 0 {
                    match entity_type {
                        "LINE" => {
                            entities.push(Entity::Line(
                                Point {
                                    x: entity_state.get(&10).ok_or_else(|| missing_tag_for_entity(10))?.parse()?,
                                    y: entity_state.get(&20).ok_or_else(|| missing_tag_for_entity(20))?.parse()?,
                                },
                                Point {
                                    x: entity_state.get(&11).ok_or_else(|| missing_tag_for_entity(11))?.parse()?,
                                    y: entity_state.get(&21).ok_or_else(|| missing_tag_for_entity(21))?.parse()?,
                                }
                            ))
                        },
                        _ => unimplemented!(),
                    }
                    entity_state.clear();
                    state = 3;
                } else {
                    entity_state.insert(tag, value);
                }
            }
            if state == 3 {
                if tag == 0 {
                    match value {
                        "LINE" => {
                            entity_type = "LINE";
                            state = 4;
                        },
                        "ENDSEC" => {
                            state = 0;
                        },
                        x => {
                            return Err(weld_err!("unsupported entity type: {}", x));
                        },
                    }
                }
            }
        }
        Ok(Drawing {
            entities,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_angle() {
        let theta = Point {
            x: 10.0,
            y: 10.0
        }.angle(&Point {
            x: 20.0,
            y: 20.0,
        });
        assert!((theta - std::f64::consts::PI / 4.0) < 0.0001);
    }
}