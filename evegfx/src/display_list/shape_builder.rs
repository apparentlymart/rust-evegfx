//! For the [`Builder::draw`](super::Builder::draw) method.

use super::Builder;
use crate::graphics::{Vertex2F, Vertex2II};

pub struct VertexBuilder<'a, DL: Builder> {
    builder: &'a mut DL,
}

impl<'a, DL: Builder> VertexBuilder<'a, DL> {
    pub(crate) fn new(builder: &'a mut DL) -> Self {
        Self { builder: builder }
    }

    pub fn vertex_2f<Pos: Into<Vertex2F>>(&mut self, pos: Pos) -> Result<(), DL::Error> {
        self.builder.vertex_2f(pos)
    }

    pub fn vertex_2ii<Pos: Into<Vertex2II>>(&mut self, pos: Pos) -> Result<(), DL::Error> {
        self.builder.vertex_2ii(pos)
    }
}
