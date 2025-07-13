use palette::{FromColor, Hsl, Mix, Srgb};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use streemap::Rect;

struct LayoutItem {
    size: f32,
    rect: Rect<f32>,
    children: Option<BTreeMap<String, LayoutItem>>,
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

fn generate_tree<I, S, N>(items: &mut [I], size_fn: S, name_fn: N) -> LayoutItem
where
    S: Fn(&I) -> f32,
    N: Fn(&I) -> &str,
{
    let mut root = LayoutItem::default();

    for (index, item) in items.iter().enumerate() {
        let name = name_fn(item);
        let size = size_fn(item);
        let mut path = name.split('/');
        // path.next(); // merge RELs

        let mut cur = &mut root;
        while let Some(part) = path.next() {
            cur.size += size;
            cur = cur.children.get_or_insert_default().entry(part.to_string()).or_default();
        }
        cur.size = size;
        cur.index = Some(index);
    }

    root
}

fn layout_tree<I, R>(items: &mut [I], root: &mut LayoutItem, set_rect_fn: &mut R)
where
    R: FnMut(&mut I, Rect<f32>),
{
    if let Some(children) = root.children.as_mut() {
        let mut v = children.values_mut().collect::<Vec<_>>();
        v.sort_by(|a, b|
		if let Some(ai) = a.index && let Some(bi) = b.index {
			ai.cmp(&bi)
		} else if a.children.is_some() && b.children.is_some() {
			b.size.total_cmp(&a.size)
		} else if a.index.is_some() {
			Ordering::Greater
		} else {
			Ordering::Less
		});
        let margin_w = root.rect.w * 0.01;
        let margin_h = root.rect.h * 0.01;
        let inlaid_rect = Rect {
            x: root.rect.x + margin_w,
            y: root.rect.y + margin_h,
            w: root.rect.w - 2. * margin_w,
            h: root.rect.h - 2. * margin_h,
        };
        streemap::binary(inlaid_rect, &mut v, |i| i.size, |i, r| i.rect = r);
        for mut child in v {
            layout_tree(items, &mut child, set_rect_fn);
        }
    }
    if let Some(index) = root.index {
        set_rect_fn(&mut items[index], root.rect);
    }
}

pub fn layout_units<T, S, N, R>(
    items: &mut [T],
    aspect: f32,
    size_fn: S,
    name_fn: N,
    mut set_rect_fn: R,
) where
    S: Fn(&T) -> f32,
    N: Fn(&T) -> &str,
    R: FnMut(&mut T, Rect<f32>),
{
    let mut tree = generate_tree(items, size_fn, name_fn);
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

pub fn color_mix(c1: Srgb, c2: Srgb, percent: f32) -> Srgb {
    c1.mix(c2, percent)
}

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
