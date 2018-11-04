#include "SlidingWindowProvider.hpp"

SlidingWindowProvider::SlidingWindowProvider(const ModuleManagerInterface& manager)
  : Module(manager)

  , cameraMatrix_(*this)
  , imageData_(*this)
  , imageSegments_(*this)
  , fieldColor_(*this)

  , minWindowSize_(*this, "minWindowSize", [this] { slidingWindowConfigChanged_.fill(true); })
  , samplePointDistance_(*this, "samplePointDistance", [this] {  slidingWindowConfigChanged_.fill(true); })
  , debugWindows_(*this, "debugWindows", [] {})
  , debugFieldColor_(*this, "debugFieldColor", [] {})
  , debugEdges_(*this, "debugEdges", [] {})

  , slidingWindows_(*this)
{
}

void SlidingWindowProvider::cycle()
{
  {
    const Chronometer time(debug(), mount_ + ".cycle_time");
    const int camera = static_cast<int>(imageData_->camera);
    // If there are no sliding windows for this camera, calculate them
    if (slidingWindows_->rows[camera].empty() || slidingWindowConfigChanged_[camera])
    {
      calculateSlidingWindows();
    }
    // If calculation was not possible, return
    if (slidingWindows_->rows[camera].empty())
    {
      return; // slidingWindows are still invalid
    }
    // Calculate the scores
    calculateScores();
    // Set the data valid
    slidingWindows_->valid = true;
  }
  sendDebug();
}

void SlidingWindowProvider::calculateSlidingWindows()
{
  assert(minWindowSize_() > 1); // modulo 1 is 0.
  if (!imageData_->is_provided || !cameraMatrix_->valid)
  {
    return;
  }
  const int camera = static_cast<int>(imageData_->camera);
  const KinematicMatrix& camera2ground = cameraMatrix_->cam2groundStand;
  const KinematicMatrix& camera2ground_inv = camera2ground.invert();
  std::vector<SlidingWindowRow>& slidingWindows = slidingWindows_->rows[camera];
  Vector2i pixel(imageData_->image422.size.x() / 2, 0);
  slidingWindows.clear();
  int currentStep = minWindowSize_();
  for (int y = imageData_->image422.size.y() - 1; y > 0; y -= currentStep)
  {
    pixel.y() = y;
    currentStep = minWindowSize_();
    Vector2f robot(Vector2f::Zero());
    if (cameraMatrix_->pixelToRobot(pixel, robot, camera2ground))
    {
      Vector2i pixelX = Vector2i::Zero();
      if (cameraMatrix_->robotToPixel({robot.x(), robot.y() - samplePointDistance_()}, pixelX,
                                      camera2ground_inv))
      {
        currentStep = std::max(pixelX.x() - pixel.x(), minWindowSize_());
      }
    }
    SlidingWindowRow row;
    row.top = y - currentStep;
    row.bottom = y;
    int startX = pixel.x() % currentStep;
    row.windows.emplace_back(Vector2i(startX - currentStep / 2, row.top),
                             Vector2i(startX, row.bottom));
    for (int x = startX; x < imageData_->image422.size.x(); x += currentStep / 2)
    {
      row.windows.emplace_back(Vector2i(x, row.top), Vector2i(x + currentStep / 2, row.bottom));
    }
    slidingWindows.push_back(row);
  }
  slidingWindowConfigChanged_[camera] = false;
}

void SlidingWindowProvider::calculateScores()
{
  const int camera = static_cast<int>(imageData_->camera);
  assert(!slidingWindows_->rows[camera].empty());
  std::vector<SlidingWindowRow>::iterator currentRow = slidingWindows_->rows[camera].end() - 1;
  for (auto& row : slidingWindows_->rows[camera])
  {
    for (auto& window : row.windows)
    {
      window.reset();
    }
  }
  for (unsigned int scanlineId = 0; scanlineId < imageSegments_->horizontalScanlines.size();
       scanlineId++)
  {
    auto& scanline = imageSegments_->horizontalScanlines[scanlineId];
    while (scanline.pos > currentRow->bottom && currentRow > slidingWindows_->rows[camera].begin())
    {
      currentRow--;
    }
    std::vector<SlidingWindow>::iterator currentWindow = currentRow->windows.begin();
    for (const auto& segment : scanline.segments)
    {
      const bool isFieldColor = fieldColor_->isFieldColor(segment.ycbcr422);
      const auto start = segment.start.x();
      const auto end = segment.end.x();
      while (currentWindow->window.bottomRight.x() < end &&
             currentWindow < currentRow->windows.end())
      {
        if (isFieldColor)
        {
          const float windowWidth = std::max<float>(
              std::min(currentWindow->window.bottomRight.x(), imageData_->image422.size.x() - 1) -
                  std::max(currentWindow->window.topLeft.x(), 0),
              1e-2f);
          const float fieldColorWidth = std::min(end, currentWindow->window.bottomRight.x()) -
                                        std::max(start, currentWindow->window.topLeft.x());
          assert(fieldColorWidth <= windowWidth);
          assert(windowWidth > 0);
          currentWindow->fieldColor += fieldColorWidth / windowWidth;
        }
        currentWindow->scanlines++;
        currentWindow++;
      }
      if (isFieldColor)
      {
        const float windowWidth = std::max<float>(
            std::min(currentWindow->window.bottomRight.x(), imageData_->image422.size.x() - 1) -
                std::max(currentWindow->window.topLeft.x(), 0),
            1e-2f);
        assert(end <= currentWindow->window.bottomRight.x());
        const float fieldColorWidth = std::min(end, currentWindow->window.bottomRight.x()) -
                                      std::max(start, currentWindow->window.topLeft.x());
        assert(fieldColorWidth <= windowWidth);
        assert(windowWidth > 0);
        currentWindow->fieldColor += fieldColorWidth / windowWidth;
      }
      if (segment.endEdgeType != EdgeType::END && segment.endEdgeType != EdgeType::BORDER)
      {
        currentWindow->edgePoints.push_back(segment.end);
      }
    }
    currentWindow->scanlines++;
  }
}

void SlidingWindowProvider::sendDebug()
{
  const auto debugImageMount = mount_ + "." + imageData_->identification + "_image";
  if (!debug().isSubscribed(debugImageMount))
  {
    return;
  }
  Image debugImage;
  imageData_->image422.to444Image(debugImage);
  int camera = static_cast<int>(imageData_->camera);
  for (const auto& row : slidingWindows_->rows[camera])
  {
    for (const auto& window : row.windows)
    {
      if (debugWindows_())
      {
        debugImage.rectangle(window.window.from422to444(), Color::WHITE);
      }
      if (debugFieldColor_())
      {
        if (window.scanlines > 0)
        {
          const float fieldColorScore = window.fieldColor / window.scanlines;
          const int barHeight =
              static_cast<float>(window.window.bottomRight.y() - window.window.topLeft.y()) *
              fieldColorScore;
          const Rectangle<int> fieldColorBar(
              {window.window.topLeft.x(), window.window.bottomRight.y() - barHeight},
              {window.window.topLeft.x(), window.window.bottomRight.y()});
          debugImage.rectangle(fieldColorBar.from422to444(), Color::PINK);
        }
      }
      if (debugEdges_())
      {
        for (const auto& edgePoint : window.edgePoints)
        {
          debugImage.circle(Image422::get444From422Vector(edgePoint), 2, Color::WHITE);
        }
        debugImage.drawString(std::to_string(window.edgePoints.size()),
                              Image422::get444From422Vector(window.window.topLeft), Color::RED);
      }
    }
  }
  debug().sendImage(debugImageMount, debugImage);
}
