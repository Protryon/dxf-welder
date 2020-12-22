use crate::dxf::*;
use crate::result::*;
use std::collections::BTreeMap;
use std::f64::consts::PI;

pub struct DxfConfig {
    pub resolution: f64, // 0.00001
    pub max_radius: f64,
    pub min_segments: usize,
}

struct Circle {
    center: Point,
    radius: f64,
}

impl Circle {
    fn get_polar_radians(&self, point: &Point) -> f64 {
        let radians = (point.y - self.center.y).atan2(point.x - self.center.x);
        if radians < 0.0 {
            return 2.0 * std::f64::consts::PI + radians;
        }
        radians
    }
}

struct Arc {
    center: Point,
    radius: f64,
    start_angle: f64,
    end_angle: f64,
}

#[derive(PartialEq)]
enum Direction {
    CounterClockwise,
    Clockwise,
    Unknown,
}

const CIRCLE_ZERO_TOLERANCE: f64 = 0.00001;

impl DxfConfig {

    // https://github.com/FormerLurker/ArcWelderPlugin/blob/master/octoprint_arc_welder/data/lib/c/arc_welder/segmented_shape.cpp#L165
    fn make_circle(&self, p1: &Point, p2: &Point, p3: &Point) -> Option<Circle> {
        let a = p1.x * (p2.y - p3.y) - p1.y * (p2.x - p3.x) + p2.x * p3.y - p3.x * p2.y;
        if a.abs() < CIRCLE_ZERO_TOLERANCE {
            return None;
        }
        let p1s = p1.x.powi(2) + p1.y.powi(2);
        let p2s = p2.x.powi(2) + p2.y.powi(2);
        let p3s = p3.x.powi(2) + p3.y.powi(2);

        let b = p1s * (p3.y - p2.y)
            + p2s * (p1.y - p3.y)
            + p3s * (p2.y - p1.y);
        
        let c = p1s * (p2.x - p3.x)
            + p2s * (p3.x - p1.x)
            + p3s * (p1.x - p2.x);
        
        let center = Point {
            x: -b / (2.0 * a),
            y: -c / (2.0 * a),
        };

        let radius = center.dist(p1);
        if radius > self.max_radius {
            return None;
        }
        Some(Circle {
            center,
            radius,
        })
    }

    // https://github.com/FormerLurker/ArcWelderPlugin/blob/master/octoprint_arc_welder/data/lib/c/arc_welder/segmented_shape.cpp#L91
    fn get_closest_perpendicular_point(&self, p1: &Point, p2: &Point, center: &Point) -> Option<Point> {
        let num = (center.x - p1.x) * (p2.x - p1.x) + (center.y - p1.y) * (p2.y - p1.y);
        let denom = (p2.x - p1.x).powi(2) + (p2.y - p1.y).powi(2);
        let t = num / denom;

        if t <= CIRCLE_ZERO_TOLERANCE || t >= (1.0 - CIRCLE_ZERO_TOLERANCE) {
            return None;
        }
        Some(Point {
            x: p1.x + t * (p2.x - p1.x),
            y: p1.y + t * (p2.y - p1.y),
        })
    }

    // https://github.com/FormerLurker/ArcWelderPlugin/blob/master/octoprint_arc_welder/data/lib/c/arc_welder/segmented_arc.cpp#L212
    fn check_chain_circle(&self, chain: &[Point], circle: &Circle/*, expected_length: f64*/) -> Option<Arc> {
        for point in chain[1..].iter() {
            let distance = circle.center.dist(point);
            let diff = (circle.radius - distance).abs();
            if diff > self.resolution {
                return None;
            }
        }
        for (i, point) in chain[0..chain.len() - 1].iter().enumerate() {
            let next = &chain[i + 1];
            if let Some(closest_point) = self.get_closest_perpendicular_point(point, next, &circle.center) {
                let distance = circle.center.dist(&closest_point);
                let diff = (circle.radius - distance).abs();
                if diff > self.resolution {
                    return None;
                }
            }
        }

        self.make_arc(circle, &chain[0], &chain[(chain.len() - 2) / 2 + 1], &chain[chain.len() - 1]/*, expected_length*/)
    }

    // https://github.com/FormerLurker/ArcWelderPlugin/blob/master/octoprint_arc_welder/data/lib/c/arc_welder/segmented_shape.cpp#L228
    fn make_arc(&self, circle: &Circle, start: &Point, mid: &Point, end: &Point/*, length: f64*/) -> Option<Arc> {
        let mut start_theta = circle.get_polar_radians(start);
        let mid_theta = circle.get_polar_radians(mid);
        let mut end_theta = circle.get_polar_radians(end);
        let mut direction = Direction::Unknown;
        if end_theta > start_theta {
            if start_theta < mid_theta && mid_theta < end_theta {
                direction = Direction::CounterClockwise;
            } else if (0.0 <= mid_theta && mid_theta < start_theta) || (end_theta < mid_theta && mid_theta < PI * 2.0) {
                direction = Direction::Clockwise;
            }
        } else if start_theta > end_theta {
            if (start_theta < mid_theta && mid_theta < 2.0 * PI) || (0.0 < mid_theta && mid_theta < end_theta) {
                direction = Direction::CounterClockwise;
            } else if end_theta < mid_theta && mid_theta < start_theta {
                direction = Direction::Clockwise;
            }
        }
        if direction == Direction::Unknown {
            return None;
        }
        // let calc_length = circle.radius * angle_radians;
        // id rather have full circles than check length properly for long runs
        // if (calc_length - length).abs() > self.resolution {
        //     return None;
        // }
        if direction == Direction::Clockwise {
            let tmp = start_theta;
            start_theta = end_theta;
            end_theta = tmp;
        }
        
        Some(Arc {
            center: circle.center.clone(),
            radius: circle.radius,
            start_angle: start_theta * 360.0 / PI / 2.0,
            end_angle: end_theta * 360.0 / PI / 2.0,
        })
    }

    fn process_chain(&self, chain: Vec<Point>) -> Result<Vec<Entity>> {
        if self.min_segments < 3 {
            return Err(weld_err!("min_segments must be >= 3"));
        }
        if chain.len() < 2 {
            return Err(weld_err!("cannot have 0 or 1 length segments"));
        } else if chain.len() == 2 {
            return Ok(vec![Entity::Line(chain[0].clone(), chain[1].clone())]);
        }
        let mut entities: Vec<Entity> = vec![];

        let mut current_arc_start = 0;
        // let mut current_arc_length: f64 = chain[0..self.min_segments - 1].windows(2).map(|p| p[0].dist(&p[1])).sum();
        let mut current_arc: Option<Arc> = None;
        let mut i = self.min_segments - 1;
        while i < chain.len() {
            // if current_arc_length < 0.0 {
            //     panic!("current_arc_start is < 0.0 {} {} {}", i, current_arc_start, current_arc_length);
            // }
            let last = &chain[i - 1];
            let point = &chain[i];
            if last == point {
                i += 1;
                continue;
            }
            // let dist = last.dist(point);
            //circlefy
            if &chain[current_arc_start] == point {
                if let Some(arc) = current_arc.take() {
                    entities.push(Entity::Circle {
                        center: arc.center,
                        radius: arc.radius,
                    });
                    current_arc_start = i + 1;
                    // current_arc_length = chain[current_arc_start..(current_arc_start + self.min_segments - 1).min(chain.len())].windows(2).map(|p| p[0].dist(&p[1])).sum();
                    i = current_arc_start + self.min_segments - 1;
                    continue;
                }
            }
            if let Some(circle) = self.make_circle(&chain[current_arc_start], &chain[current_arc_start + (i - current_arc_start - 2) / 2 + 1], point) {
                if let Some(arc) = self.check_chain_circle(&chain[current_arc_start..i + 1], &circle/*, current_arc_length + dist*/) {
                    // current_arc_length += dist;
                    current_arc = Some(arc);
                    i += 1;
                    continue;
                }
            }

            if let Some(arc) = current_arc.take() {
                entities.push(Entity::Arc {
                    center: arc.center,
                    radius: arc.radius,
                    start_angle: arc.start_angle,
                    end_angle: arc.end_angle,
                });
                current_arc_start = i;
                // current_arc_length = chain[current_arc_start..(current_arc_start + self.min_segments - 1).min(chain.len())].windows(2).map(|p| p[0].dist(&p[1])).sum();
                i = current_arc_start + self.min_segments - 1;
                continue;
            } else {
                entities.push(Entity::Line(chain[current_arc_start].clone(), chain[current_arc_start + 1].clone()));
                // current_arc_length -= chain[current_arc_start].dist(&chain[current_arc_start + 1]);
                current_arc_start += 1;
                if i - current_arc_start >= self.min_segments {
                    continue;
                }
            }

            // current_arc_length += dist;
            i += 1;
        }
        if let Some(arc) = current_arc.take() {
            entities.push(Entity::Arc {
                center: arc.center,
                radius: arc.radius,
                start_angle: arc.start_angle,
                end_angle: arc.end_angle,
            });
        } else {
            for points in chain[current_arc_start..chain.len()].windows(2) {
                entities.push(Entity::Line(points[0].clone(), points[1].clone()));
            }
        }

        Ok(entities)
    }

    pub fn process_drawing(&self, drawing: Drawing) -> Result<Drawing> {
        let mut src_dest: BTreeMap<Point, Point> = BTreeMap::new();

        let mut new_entities = vec![];
        for entity in drawing.entities.into_iter() {
            match entity {
                Entity::Line(from, to) => {
                    src_dest.insert(from, to);
                },
                x => return Err(weld_err!("cannot process dxf with non-line: {:?}", &x)),
            }
        }
        
        let mut chains: Vec<Vec<Point>> = vec![];
        while src_dest.len() > 0 {
            let mut chain = vec![];
            let (first_from, mut next) = src_dest.iter().next().map(|(p1, p2)| (p1.clone(), p2.clone())).unwrap();
            src_dest.remove(&first_from).unwrap();

            chain.push(first_from.clone());
            while let Some(point) = src_dest.remove(&next) {
                chain.push(next);
                next = point;
            }
            chain.push(next);
            chains.push(chain);
        }
        for chain in chains.into_iter() {
            let output = self.process_chain(chain)?;
            new_entities.extend(output);
        }
        Ok(Drawing {
            entities: new_entities,
        })
    }
}

