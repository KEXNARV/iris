//! Los estilos de /theme: cada uno reparte y dibuja la pantalla a su manera alrededor del núcleo.

mod clasico;
mod propuesta;
mod cabina;
mod cine;
mod fosforo;
mod radar;
mod editorial;
mod zen;
mod bitacora;
mod tablero;

pub(in crate::ui) use clasico::*;
pub(in crate::ui) use propuesta::*;
pub(in crate::ui) use cabina::*;
pub(in crate::ui) use cine::*;
pub(in crate::ui) use fosforo::*;
pub(in crate::ui) use radar::*;
pub(in crate::ui) use editorial::*;
pub(in crate::ui) use zen::*;
pub(in crate::ui) use bitacora::*;
pub(in crate::ui) use tablero::*;
