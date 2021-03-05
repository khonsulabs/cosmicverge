use std::{collections::HashMap, f32::consts::PI};

use cosmicverge_shared::euclid::{Point2D, Rotation2D, Size2D, Vector2D};
use kludgine::prelude::*;

use super::data_model::{Cluster, NodeId, SolarSystemServerId};

#[derive(Debug, Default)]
pub struct ClusterMap {
    cluster: Handle<Cluster>,
    last_layout: Option<MapLayout>,
    hovered: Option<MapLayoutEntity>,
}

#[derive(Debug, Clone)]
pub enum Command {}

#[async_trait]
impl Component for ClusterMap {
    async fn update(&mut self, context: &mut Context) -> KludgineResult<()> {
        // Create or remove any servers. If any nodes have changed, we need to re-layout
        let cluster = self.cluster.read().await;
        let mut needs_layout = self.last_layout.is_none();
        if let Some(last_layout) = &self.last_layout {
            for node in cluster.nodes.values() {
                if !last_layout
                    .entities
                    .contains_key(&MapLayoutEntity::Node(node.id))
                {
                    needs_layout = true;
                    break;
                }
            }

            if !needs_layout {
                for server in cluster.servers.values() {
                    if !last_layout
                        .entities
                        .contains_key(&MapLayoutEntity::SystemServer(server.id))
                    {
                        needs_layout = true;
                        break;
                    }
                }
            }
        }

        if needs_layout {
            self.last_layout = Some(self.update_layout(&cluster).await);
            context.set_needs_redraw().await;
        }

        Ok(())
    }

    async fn content_size(
        &self,
        _context: &mut StyledContext,
        _constraints: &Size<Option<f32>, Scaled>,
    ) -> KludgineResult<Size<f32, Scaled>> {
        Ok(self
            .last_layout
            .as_ref()
            .map(|l| l.size)
            .unwrap_or_default())
    }

    async fn mouse_moved(
        &mut self,
        context: &mut Context,
        window_position: Option<Point<f32, Scaled>>,
    ) -> KludgineResult<()> {
        if let Some(position) = window_position {
            if let Some(map_layout) = &self.last_layout {
                let last_layout = self.last_layout(context).await;
                let position = position - last_layout.inner_bounds().origin.to_vector();
                for (id, location) in &map_layout.entities {
                    if location.center.distance_to(position) < location.radius.get() {
                        if self.hovered != Some(*id) {
                            self.hovered = Some(*id);
                            context.set_needs_redraw().await;
                        }
                        return Ok(());
                    }
                }
            }
        }

        self.unhover_if_needed(context).await;

        Ok(())
    }

    async fn unhovered(&mut self, context: &mut Context) -> KludgineResult<()> {
        self.unhover_if_needed(context).await;

        Ok(())
    }

    async fn render(&mut self, context: &mut StyledContext, layout: &Layout) -> KludgineResult<()> {
        let bounds = layout.inner_bounds();

        if let Some(map_layout) = &self.last_layout {
            for (map_layout_id, location) in &map_layout.entities {
                let (id, color) = match map_layout_id {
                    MapLayoutEntity::Node(node_id) => (*node_id, Color::LIGHTGREY),
                    MapLayoutEntity::SystemServer(server_id) => (*server_id, Color::LIGHTGREEN),
                };

                self.render_node(
                    &id.to_string(),
                    context,
                    bounds.origin,
                    location,
                    color,
                    self.hovered == Some(*map_layout_id),
                )
                .await?
            }
        }

        Ok(())
    }
}

#[async_trait]
impl InteractiveComponent for ClusterMap {
    type Message = ();
    type Command = Command;
    type Event = ();
}

impl ClusterMap {
    pub const fn new(cluster: Handle<Cluster>) -> Self {
        Self {
            cluster,
            last_layout: None,
            hovered: None,
        }
    }

    async fn unhover_if_needed(&mut self, context: &mut Context) {
        if self.hovered.is_some() {
            self.hovered = None;
            context.set_needs_redraw().await;
        }
    }

    async fn update_layout(&self, cluster: &Cluster) -> MapLayout {
        // Lay the nodes out in a circular pattern.
        let mut servers_by_node = HashMap::new();
        for server in cluster.servers.values() {
            let servers = servers_by_node
                .entry(server.node_id)
                .or_insert_with(Vec::new);
            servers.push(server.id);
        }

        // This is kind of ugly, but the idea is to create a circle with the
        // radius `circle_radius`, and then distributed evenly along that circle
        // will be Nodes, which will have a circle of Servers around them. The
        // math here attempts to create a reasonable amount of spacing.
        const ENTITY_RADIUS: Points = Points::new(25.);
        let maximum_node_radius = ENTITY_RADIUS * 3.;
        let angle_between_nodes = PI * 2. / cluster.nodes.len() as f32;
        let circle_radius = maximum_node_radius * 3.;

        let maximum_radius = circle_radius + maximum_node_radius;
        let content_size = Size::from_lengths(maximum_radius * 2., maximum_radius * 2.);
        let center = content_size.to_vector() / 2.;

        let mut layout = MapLayout::default();
        // Sort the nodes first before laying out to ensure nodes don't move around between frames
        let mut nodes = cluster.nodes.iter().collect::<Vec<_>>();
        nodes.sort_by_key(|n| n.0);
        for (index, (node_id, node)) in nodes.into_iter().enumerate() {
            let node_center =
                Rotation2D::<f32, Scaled, Scaled>::radians(angle_between_nodes * index as f32)
                    .transform_point(Point::from_lengths(circle_radius, Length::default()))
                    + center;
            layout.entities.insert(
                MapLayoutEntity::Node(node.id),
                MapLayoutInfo {
                    center: node_center,
                    radius: ENTITY_RADIUS,
                },
            );

            if let Some(servers) = servers_by_node.get_mut(node_id) {
                servers.sort_unstable();
                let angle_between_servers = PI * 2. / servers.len() as f32;
                for (index, server_id) in servers.iter().enumerate() {
                    let server_center = node_center
                        + Rotation2D::<f32, Scaled, Scaled>::radians(
                            angle_between_servers * index as f32,
                        )
                        .transform_vector(Vector2D::from_lengths(
                            maximum_node_radius - ENTITY_RADIUS / 2.,
                            Length::default(),
                        ));
                    layout.entities.insert(
                        MapLayoutEntity::SystemServer(*server_id),
                        MapLayoutInfo {
                            center: server_center,
                            radius: ENTITY_RADIUS,
                        },
                    );
                }
            }
        }

        layout
    }

    async fn render_node(
        &self,
        text: &str,
        context: &mut StyledContext,
        origin: Point<f32, Scaled>,
        layout: &MapLayoutInfo,
        color: Color,
        hovered: bool,
    ) -> KludgineResult<()> {
        let center = origin + layout.center.to_vector();
        let mut shape = Shape::circle(center, layout.radius).fill(Fill::new(color));
        if hovered {
            shape = shape.stroke(Stroke::new(Color::BLUE))
        }
        shape.render_at(Point2D::default(), context.scene()).await;

        let measured = Text::span(text, context.effective_style()?.clone())
            .wrap(context.scene(), TextWrap::NoWrap)
            .await?;
        let scale = context.scene().scale_factor().await;
        let size = measured.size().await / scale;
        let top_left = center - size.to_vector() / 2.;
        measured.render(context.scene(), top_left, true).await?;

        Ok(())
    }
}

#[derive(Default, Debug)]
struct MapLayout {
    entities: HashMap<MapLayoutEntity, MapLayoutInfo>,
    size: Size2D<f32, Scaled>,
}

#[derive(Debug)]
struct MapLayoutInfo {
    center: Point<f32, Scaled>,
    radius: Length<f32, Scaled>,
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
enum MapLayoutEntity {
    Node(NodeId),
    SystemServer(SolarSystemServerId),
}
