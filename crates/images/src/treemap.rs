use std::{cmp::Ordering, collections::BTreeMap};

use palette::{FromColor, Hsl, Mix, Srgb};
use streemap::Rect;

struct LayoutItem {
    size: f32,
    rect: Rect<f32>,
    children: Option<BTreeMap<(String, Option<usize>), LayoutItem>>,
    index: Option<usize>,
}

impl Default for LayoutItem {
    fn default() -> Self {
        LayoutItem {
            size: 0.,
            rect: Rect { x: 0., y: 0., w: 0., h: 0. },
            children: None,
            index: None,
        }
    }
}

fn generate_tree<I, S, N>(
    items: &mut [I],
    merge_root_folders: bool,
    size_fn: S,
    name_fn: N,
) -> LayoutItem
where
    S: Fn(&I) -> f32,
    N: Fn(&I) -> &str,
{
    let mut root = LayoutItem::default();

    for (index, item) in items.iter().enumerate() {
        let name = name_fn(item);
        let size = size_fn(item);
        let mut path = name.split('/').peekable();

        let mut cur = &mut root;
        let mut at_root = true;
        while let Some(part) = path.next() {
            let mut discriminator = None;
            if path.peek().is_none() {
                // Distinguish files from folders, and files from other files with the same name.
                discriminator = Some(index);
            } else if at_root && merge_root_folders {
                at_root = false;
                continue;
            }
            cur.size += size;
            cur = cur
                .children
                .get_or_insert_default()
                .entry((part.to_string(), discriminator))
                .or_default();
            at_root = false;
        }
        cur.size = size;
        cur.index = Some(index);
    }

    root
}

fn inlay_rect(r: Rect<f32>, margin: f32) -> Rect<f32> {
    let margin_w = r.w * margin;
    let margin_h = r.h * margin;
    Rect { x: r.x + margin_w, y: r.y + margin_h, w: r.w - margin_w * 2.0, h: r.h - margin_h * 2.0 }
}

fn layout_tree<I, R>(items: &mut [I], root: &mut LayoutItem, set_rect_fn: &mut R)
where R: FnMut(&mut I, Rect<f32>) {
    if let Some(children) = root.children.as_mut() {
        let mut v = children.values_mut().collect::<Vec<_>>();
        v.sort_by(|a, b| {
            if let Some(ai) = a.index
                && let Some(bi) = b.index
            {
                ai.cmp(&bi)
            } else if a.children.is_some() && b.children.is_some() {
                b.size.total_cmp(&a.size)
            } else if a.index.is_some() {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        });
        streemap::binary(inlay_rect(root.rect, 0.02), &mut v, |i| i.size, |i, r| i.rect = r);
        for mut child in v {
            layout_tree(items, &mut child, set_rect_fn);
        }
    }
    if let Some(index) = root.index {
        set_rect_fn(&mut items[index], inlay_rect(root.rect, 0.005));
    }
}

pub fn layout_units<T, S, N, R>(
    items: &mut [T],
    aspect: f32,
    merge_root_folders: bool,
    size_fn: S,
    name_fn: N,
    mut set_rect_fn: R,
) where
    S: Fn(&T) -> f32,
    N: Fn(&T) -> &str,
    R: FnMut(&mut T, Rect<f32>),
{
    let mut tree = generate_tree(items, merge_root_folders, size_fn, name_fn);
    tree.rect = if aspect > 1.0 {
        Rect::from_size(1.0, 1.0 / aspect)
    } else {
        Rect::from_size(aspect, 1.0)
    };
    layout_tree(items, &mut tree, &mut |item, mut rect| {
        if aspect > 1.0 {
            rect.y *= aspect;
            rect.h *= aspect;
        } else {
            rect.x /= aspect;
            rect.w /= aspect;
        }
        set_rect_fn(item, rect);
    });
}

pub fn hsl(h: u16, s: u8, l: u8) -> Srgb {
    let hsl = Hsl::new(h as f32, s as f32 / 100.0, l as f32 / 100.0);
    Srgb::from_color(hsl)
}

pub fn color_mix(c1: Srgb, c2: Srgb, percent: f32) -> Srgb { c1.mix(c2, percent) }

pub fn unit_color(fuzzy_match_percent: f32) -> String {
    html_color(if fuzzy_match_percent == 100.0 {
        hsl(120, 100, 39)
    } else {
        let nonmatch = hsl(221, 0, 21);
        let nearmatch = hsl(221, 100, 35);
        nonmatch.mix(nearmatch, fuzzy_match_percent / 100.0)
    })
}

pub fn html_color(c: Srgb) -> String {
    let (r, g, b) = c.into_components();
    format!("#{:02x}{:02x}{:02x}", (r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}
