#include "BallSeedsProvider.hpp"
#include "Tools/Chronometer.hpp"


BallSeedsProvider::BallSeedsProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , cameraMatrix_(*this)
  , imageData_(*this)
  , imageSegments_(*this)
  , fieldBorder_(*this)
  , fieldDimensions_(*this)

  , minSeedBrightDiff_(*this, "minSeedBrightDiff", [] {})
  , seedBrightMin_(*this, "seedBrightMin", [] {})
  , seedBrightScore_(*this, "seedBrightScore", [] {})
  , seedDark_(*this, "seedDark", [] {})
  , seedRadiusRatioMin_(*this, "seedRadiusRatioMin", [] {})
  , seedRadiusRatioMax_(*this, "seedRadiusRatioMax", [] {})

  , ballSeeds_(*this)
{
}

void BallSeedsProvider::cycle()
{
  findSeeds(ballSeeds_->seeds);
}

void BallSeedsProvider::findSeeds(std::vector<BallSeeds::Seed>& seeds) const
{
  for (auto& scanline : imageSegments_->verticalScanlines)
  {
    unsigned long regionCount = scanline.segments.size();
    for (unsigned int i = 0; i < regionCount; i++)
    {
      if (scanline.segments[i].ycbcr422.y1_ > seedDark_())
      {
        continue;
      }
      if (!fieldBorder_->isInsideField(scanline.segments[i].start))
      {
        continue;
      }
      const Vector2i seed = (scanline.segments[i].start + scanline.segments[i].end) / 2;
      int pixelRadius = 0;
      cameraMatrix_->getPixelRadius(imageData_->image422.size, seed,
                                    fieldDimensions_->ballDiameter / 2, pixelRadius);

      const float regionSize =
          static_cast<float>(scanline.segments[i].end.y() - scanline.segments[i].start.y()) /
          pixelRadius;
      if (regionSize < seedRadiusRatioMin_() || regionSize > seedRadiusRatioMax_())
      {
        continue;
      }
      const std::array<Vector2i, 8> directions = {
          {{-1, -2}, {0, -2}, {1, -2}, {-1, 0}, {1, 0}, {-1, 2}, {0, 2}, {1, 2}}};

      int seedY = imageData_->image422[seed].y1_;
      bool allBrighter = true;
      bool allInside = true;
      int score = 0;
      for (auto& d : directions)
      {
        // Move from seed into direction * pixelRadius * (10/25)
        // 10/25 is a well working magic number
        // 422 conversion is done by multiplying d.y with two (see above) and dividing the magic
        // number
        const Vector2i& point = seed + (d * pixelRadius * 5 / 25);
        if (!imageData_->image422.isInside(point))
        {
          allInside = false;
          break;
        }
        const int pointY = imageData_->image422[point].y1_;
        if (pointY - seedY < seedBrightMin_())
        {
          allBrighter = false;
          break;
        }
        if (pointY - seedY > minSeedBrightDiff_())
        {
          score++;
        }
      }

      if (!allBrighter || !allInside)
      {
        continue;
      }

      if (score < seedBrightScore_())
      {
        continue;
      }
      seeds.emplace_back(seed, pixelRadius);
    }
  }
}
