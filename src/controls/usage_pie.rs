use cosmic::Element;
use cosmic::iced::widget as iced_widget;
use cosmic::iced::{Alignment, Color, Font, Length, Pixels, Point, Rectangle, mouse};
use cosmic::widget::{self};
use storage_types::bytes_to_pretty;

use crate::app::Message;

/// Color palette for disk pie chart segments.
const SEGMENT_COLORS: [Color; 10] = [
    Color {
        r: 0.267,
        g: 0.557,
        b: 0.886,
        a: 1.0,
    }, // Blue
    Color {
        r: 0.180,
        g: 0.729,
        b: 0.412,
        a: 1.0,
    }, // Green
    Color {
        r: 0.937,
        g: 0.325,
        b: 0.325,
        a: 1.0,
    }, // Red
    Color {
        r: 0.980,
        g: 0.702,
        b: 0.204,
        a: 1.0,
    }, // Yellow
    Color {
        r: 0.608,
        g: 0.349,
        b: 0.714,
        a: 1.0,
    }, // Purple
    Color {
        r: 0.902,
        g: 0.494,
        b: 0.133,
        a: 1.0,
    }, // Orange
    Color {
        r: 0.204,
        g: 0.698,
        b: 0.714,
        a: 1.0,
    }, // Teal
    Color {
        r: 0.878,
        g: 0.431,
        b: 0.557,
        a: 1.0,
    }, // Pink
    Color {
        r: 0.451,
        g: 0.486,
        b: 0.914,
        a: 1.0,
    }, // Indigo
    Color {
        r: 0.663,
        g: 0.792,
        b: 0.255,
        a: 1.0,
    }, // Lime
];

/// Returns the color for a given segment index.
pub fn segment_color(index: usize) -> Color {
    SEGMENT_COLORS[index % SEGMENT_COLORS.len()]
}

/// Data for a single segment in the disk usage pie chart.
pub struct PieSegmentData {
    pub name: String,
    pub used: u64,
}

/// Canvas program for drawing a multi-segment donut chart with centered percentage text.
struct DiskPieProgram {
    /// (start_angle, sweep_angle, color) for each segment.
    arcs: Vec<(f32, f32, Color)>,
    ring_radius: f32,
    ring_width: f32,
    /// Percentage text to render at the center (e.g. "42%").
    percent_text: String,
}

impl<M> iced_widget::canvas::Program<M, cosmic::Theme, cosmic::Renderer> for DiskPieProgram {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &cosmic::Renderer,
        theme: &cosmic::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<iced_widget::canvas::Geometry<cosmic::Renderer>> {
        use iced_widget::canvas::{Frame, LineCap, Path, Stroke, Text};

        let mut frame = Frame::new(renderer, bounds.size());
        let center = frame.center();

        // Background ring (shows through for free/unpartitioned space)
        let bg_circle = Path::circle(center, self.ring_radius);
        let divider_color: Color = theme.cosmic().background(false).component.divider.into();
        frame.stroke(
            &bg_circle,
            Stroke::default()
                .with_width(self.ring_width)
                .with_color(divider_color),
        );

        // Draw each partition segment as a thick arc
        for &(start, sweep, color) in &self.arcs {
            if sweep < 0.001 {
                continue;
            }
            let path = Path::new(|builder| {
                let steps = (sweep * 30.0).max(2.0) as usize;
                for i in 0..=steps {
                    let angle = start + sweep * (i as f32 / steps as f32);
                    let point = Point::new(
                        center.x + self.ring_radius * angle.cos(),
                        center.y + self.ring_radius * angle.sin(),
                    );
                    if i == 0 {
                        builder.move_to(point);
                    } else {
                        builder.line_to(point);
                    }
                }
            });
            frame.stroke(
                &path,
                Stroke::default()
                    .with_width(self.ring_width)
                    .with_color(color)
                    .with_line_cap(LineCap::Butt),
            );
        }

        // Draw percentage text centered in the donut hole
        let text_color: Color = theme.cosmic().background(false).on.into();
        frame.fill_text(Text {
            content: self.percent_text.clone(),
            position: center,
            color: text_color,
            size: Pixels(14.0),
            font: Font {
                weight: cosmic::iced::font::Weight::Semibold,
                ..Default::default()
            },
            align_x: cosmic::iced::widget::text::Alignment::Center,
            align_y: cosmic::iced::alignment::Vertical::Center,
            ..Default::default()
        });

        vec![frame.into_geometry()]
    }
}

/// Renders a multi-segment donut pie chart for the disk header showing partition usage,
/// with an optional legend below listing each partition's color, name, and used/total space.
pub fn disk_usage_pie<'a>(
    segments: &[PieSegmentData],
    total_disk_size: u64,
    used: u64,
    show_legend: bool,
) -> Element<'a, Message> {
    use std::f32::consts::PI;

    let pie_size = 96.0_f32;
    let ring_radius = (pie_size / 2.0) - 6.0;
    let ring_width = 10.0;

    let denom = total_disk_size.max(1) as f64;

    let mut arcs = Vec::new();
    let mut current_angle = -PI / 2.0; // Start from 12 o'clock

    for (i, seg) in segments.iter().enumerate() {
        let fraction = seg.used as f64 / denom;
        let sweep = (fraction * 2.0 * PI as f64) as f32;
        let color = segment_color(i);
        arcs.push((current_angle, sweep, color));
        current_angle += sweep;
    }

    let percent = if total_disk_size > 0 {
        ((used as f64 / total_disk_size as f64) * 100.0) as u32
    } else {
        0
    };

    let program = DiskPieProgram {
        arcs,
        ring_radius,
        ring_width,
        percent_text: format!("{}%", percent),
    };

    let canvas_widget: Element<'a, Message> = iced_widget::canvas::Canvas::new(program)
        .width(Length::Fixed(pie_size))
        .height(Length::Fixed(pie_size))
        .into();

    let used_text = format!(
        "{} / {}",
        bytes_to_pretty(&used, false),
        bytes_to_pretty(&total_disk_size, false)
    );

    if show_legend {
        // Legend: color swatch | partition name and used
        let mut legend_items: Vec<Element<'a, Message>> = Vec::new();
        for (i, seg) in segments.iter().enumerate() {
            let color = segment_color(i);
            let swatch: Element<'a, Message> =
                widget::container(widget::Space::new().width(10.0).height(10.0))
                    .style(
                        move |_theme: &cosmic::Theme| iced_widget::container::Style {
                            background: Some(cosmic::iced::Background::Color(color)),
                            border: cosmic::iced::Border {
                                radius: 2.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                    )
                    .into();

            let label = iced_widget::column![
                widget::text::caption(seg.name.clone()).font(cosmic::iced::font::Font {
                    weight: cosmic::iced::font::Weight::Bold,
                    ..Default::default()
                }),
                widget::text::caption(bytes_to_pretty(&seg.used, false)),
            ]
            .spacing(2);

            legend_items.push(
                iced_widget::row![swatch, label]
                    .spacing(12)
                    .align_y(Alignment::Center)
                    .into(),
            );
        }

        let legend = iced_widget::Column::from_vec(legend_items)
            .spacing(2)
            .align_x(Alignment::Start);

        let pie_with_text =
            iced_widget::column![canvas_widget, widget::text::caption(used_text).center(),]
                .spacing(8)
                .align_x(Alignment::Center);

        // Row layout: legend | pie
        iced_widget::row![legend, pie_with_text,]
            .spacing(15)
            .align_y(Alignment::Center)
            .into()
    } else {
        iced_widget::column![canvas_widget, widget::text::caption(used_text).center(),]
            .spacing(8)
            .align_x(Alignment::Center)
            .into()
    }
}
