#pragma once

#include <memory>
#include <stb_image.h>

namespace Hulks::GridCropper
{

  struct Image
  {
    std::unique_ptr<stbi_uc, void (*)(void*)> data{nullptr, &stbi_image_free};
    int width{0};
    int height{0};
    int colorsPerPixel{0};
  };

} // namespace Hulks::GridCropper
