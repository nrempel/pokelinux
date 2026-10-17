// Structural control: actual existing headers/types only; no instances or link.
#include "render/Texture.hpp"
#include "render/SHMSnapshot.hpp"
#include <cstdint>
#include <type_traits>
using Texture = Render::ITexture;
using Snapshot = Render::CSHMSnapshot;
static_assert(std::is_abstract_v<Texture>);
static_assert(std::has_virtual_destructor_v<Texture>);
static_assert(std::is_same_v<decltype(Texture::m_texID), GLuint>);
static_assert(std::is_same_v<decltype(Texture::m_drmFormat), uint32_t>);
static_assert(std::is_same_v<decltype(Texture::m_opaque), bool>);
static_assert(std::is_same_v<decltype(Texture::magFilter), GLenum>);
static_assert(std::is_same_v<decltype(Texture::minFilter), GLenum>);
static_assert(std::is_same_v<decltype(&Texture::allocate), void (Texture::*)(const Vector2D&, uint32_t)>);
static_assert(std::is_same_v<decltype(&Texture::update), void (Texture::*)(uint32_t, const uint8_t*, uint32_t, const CRegion&)>);
static_assert(std::is_copy_constructible_v<Snapshot>);
static_assert(std::is_nothrow_move_constructible_v<Snapshot>);
static_assert(std::is_same_v<decltype(&Snapshot::capture), int32_t (*)(PrShmDescription, const uint8_t*, uint64_t, Snapshot&) noexcept>);
int main() { return 0; }
