#include "Vision/RobotDetection/RobotDetection.hpp"
#include "Framework/Log/Log.hpp"
#include "Tools/Chronometer.hpp"
#include "Tools/Math/Statistics.hpp"

RobotDetection::RobotDetection(const ModuleManagerInterface& manager)
  : Module(manager)
  , bodyPose_(*this)
  , cameraMatrix_(*this)
  , fieldBorder_(*this)
  , fieldDimensions_(*this)
  , imageData_(*this)
  , imageSegments_(*this)

  , detectionBoxHeight_(*this, "detectionBoxHeight", [] {})
  , detectionBoxWidth_(*this, "detectionBoxWidth", [] {})
  , minConsecutiveSegments_(*this, "minConsecutiveSegments", [] {})
  , minEdgePointsInCandidateBox_(*this, "minEdgePointsInCandidateBox", [] {})
  , drawEdgePoints_(*this, "drawEdgePoints", [] {})
  , drawSeeds_(*this, "drawSeeds", [] {})
  , drawAcceptedCandidates_(*this, "drawAcceptedCandidates", [] {})
  , drawCutCandidates_(*this, "drawCutCandidates", [] {})
  , drawRejectedCandidates_(*this, "drawRejectedCandidates", [] {})
  , drawWindows_(*this, "drawWindows", [] {})

  , robotData_(*this)
{
}

void RobotDetection::cycle()
{
  {
    Chronometer time(debug(), mount_ + "." + imageData_->identification + "_cycle_time");
    if (!imageSegments_->valid || bodyPose_->fallen || !cameraMatrix_->valid || !bodyPose_->upright)
    {
      return;
    }
    columns_.clear();
    columns_.reserve(imageSegments_->verticalScanlines.size());
    debugAcceptedBoxes_.clear();
    debugCutBoxes_.clear();
    debugRejectedBoxes_.clear();
    debugWindows_.clear();
    setupColumns();
    medianSeeds();
    findRobots();
  }
  sendRobotPositionImageForDebug();
  sendHistogramImageForDebug();
}


void RobotDetection::setupColumns()
{
  for (const auto& scanline : imageSegments_->verticalScanlines)
  {
    columns_.emplace_back(Vector2i(scanline.pos, 0));
    int consecutiveSegmentsCount = 0;
    for (const auto& segment : scanline.segments)
    {
      if (!fieldBorder_->isInsideField(segment.end))
      {
        continue;
      }
      if (segment.field > 0.f)
      {
        consecutiveSegmentsCount = 0;
        continue;
      }
      consecutiveSegmentsCount++;
      if (consecutiveSegmentsCount > minConsecutiveSegments_())
      {
        columns_.back().edgePointsY.push_back(segment.end.y());
        columns_.back().seedPosition = segment.end;
      }
    }
  }
}

void RobotDetection::medianSeeds()
{
  // calculate the median and write it to the seedPositionYMedian buffer
  for (size_t i = 1; i < columns_.size() - 1; i++)
  {
    const int lastSeedPositionY = columns_[i - 1].seedPosition.y();
    const int thisSeedPositionY = columns_[i].seedPosition.y();
    const int nextSeedPositionY = columns_[i + 1].seedPosition.y();
    if (lastSeedPositionY == 0 || thisSeedPositionY == 0 || nextSeedPositionY == 0)
    {
      columns_[i].seedPositionYMedian = 0;
    }
    else
    {
      columns_[i].seedPositionYMedian =
          Statistics::median(lastSeedPositionY, thisSeedPositionY, nextSeedPositionY);
    }
  }
  // overwrite the seed position with the buffered median seed position
  for (auto& column : columns_)
  {
    column.seedPosition.y() = column.seedPositionYMedian;
  }
}

Column* RobotDetection::getColumnWithNearestSeed()
{
  int maxY = 0;
  Column* bestColumn = nullptr;
  for (auto& column : columns_)
  {
    if (column.visited || column.deleted)
    {
      continue;
    }
    const int& seedPositionY = column.seedPosition.y();
    if (seedPositionY > maxY)
    {
      maxY = seedPositionY;
      bestColumn = &column;
    }
  }
  return bestColumn;
}

bool RobotDetection::findBestCandidate(const Vector2i& seed, Candidate& candidate)
{
  const std::optional<Vector2f> robotCoordinates = cameraMatrix_->pixelToRobot(seed);
  if (!robotCoordinates.has_value())
  {
    Log<M_VISION>(LogLevel::WARNING) << "RobotDetection: Projection to robot coordinates failed";
    return false;
  }

  const Vector3f cameraCoordinates =
      cameraMatrix_->camera2groundInv * Vector3f(robotCoordinates->x(), robotCoordinates->y(), 0);

  const Vector3f topLeft(cameraCoordinates +
                         Vector3f(0, detectionBoxWidth_(), detectionBoxHeight_()));
  const std::optional<Vector2i> pixelTopLeft = cameraMatrix_->cameraToPixel(topLeft);
  if (!pixelTopLeft.has_value())
  {
    Log<M_VISION>(LogLevel::WARNING) << "RobotDetection: Projection to pixels failed";
    return false;
  }
  const Vector2i boxSize = (seed - pixelTopLeft.value()).cwiseAbs();

  Rectangle<int> window(
      Vector2i(std::max(seed.x() - boxSize.x(), 0), pixelTopLeft->y()),
      Vector2i(std::min(seed.x() + boxSize.x(), imageData_->image422.size.x()), seed.y()));
  debugWindows_.push_back(window);

  const auto columnWidth =
      static_cast<float>(imageData_->image422.size.x()) / static_cast<float>(columns_.size());
  const unsigned int boxSizeXInColumns = boxSize.x() / static_cast<unsigned int>(columnWidth);

  std::vector<unsigned int> integral;
  integral.reserve(static_cast<std::size_t>(window.size().x() / columnWidth));
  unsigned int integralBuffer = 0;
  // pad integral vector with zeros to account for boundary conditions on the left
  for (unsigned int i = 0; i <= boxSizeXInColumns; i++)
  {
    integral.push_back(0);
  }
  for (const auto& column : columns_)
  {
    if (column.x() < window.topLeft.x())
    {
      continue;
    }
    if (column.x() > window.bottomRight.x())
    {
      break;
    }
    int sumEdgesInColumn = 0;
    if (!column.deleted)
    {
      sumEdgesInColumn =
          std::count_if(column.edgePointsY.begin(), column.edgePointsY.end(), [&window](int y) {
            return ((y > window.topLeft.y()) && (y < window.bottomRight.y()));
          });
    }
    integralBuffer += sumEdgesInColumn;
    integral.push_back(integralBuffer);
  }
  // pad integral vector with last value to account for boundary conditions on the right
  for (unsigned int i = 0; i <= boxSizeXInColumns; i++)
  {
    integral.push_back(integral.back());
  }

  unsigned int maximumValue = 0;
  size_t maximumIndexLeft = 0;
  size_t maximumIndexRight = 0;
  for (size_t i = 0; i < integral.size() - boxSizeXInColumns; i++)
  {
    const size_t right = i + boxSizeXInColumns;
    const unsigned int value = integral[right] - integral[i];
    if (value > maximumValue)
    {
      maximumValue = value;
      maximumIndexLeft = i;
      maximumIndexRight = i;
    }
    else if (value == maximumValue)
    {
      maximumIndexRight = i;
    }
  }
  const size_t maximumIndex = (maximumIndexLeft + maximumIndexRight) / 2;
  candidate.numberEdgePoints = maximumValue;
  const int candidateBoxLeft = window.topLeft.x() +
                               static_cast<int>(maximumIndex) * static_cast<int>(columnWidth) -
                               boxSize.x();
  const int candidateBoxRight =
      window.topLeft.x() + static_cast<int>(maximumIndex) * static_cast<int>(columnWidth);
  candidate.box = Rectangle<int>(Vector2i(candidateBoxLeft, window.topLeft.y()),
                                 Vector2i(candidateBoxRight, window.bottomRight.y()));
  return true;
}

void RobotDetection::deleteColumns(const Candidate& candidate, const float deletePaddingFactor)
{
  for (auto& column : columns_)
  {
    const int numberPaddingColumns =
        static_cast<int>(static_cast<float>(candidate.box.size().x()) * deletePaddingFactor);
    if (column.x() < candidate.box.topLeft.x() - numberPaddingColumns)
    {
      continue;
    }
    if (column.x() > candidate.box.bottomRight.x() + numberPaddingColumns)
    {
      break;
    }
    column.deleted = true;
  }
}

void RobotDetection::findRobots()
{
  while (true)
  {
    Column* currentColumn = getColumnWithNearestSeed();
    if (!currentColumn)
    {
      break;
    }
    currentColumn->visited = true;
    Candidate candidate;
    if (!findBestCandidate(currentColumn->seedPosition, candidate))
    {
      continue;
    }
    if (candidate.numberEdgePoints >= minEdgePointsInCandidateBox_())
    {
      // consider robot candidates as cut i.e. seeds are at the bottom of the image
      if (candidate.box.bottomRight.y() == imageData_->image422.size.y() - 1)
      {
        // delete additional columns for cut robots as the projection of robots at the bottom of the
        // image leads to candidate boxes that are too small which can lead to false positives in
        // the robot's hands
        deleteColumns(candidate, 1);
        debugCutBoxes_.emplace_back(candidate.box.get444from422(), candidate.numberEdgePoints);
        continue;
      }
      deleteColumns(candidate, 0.5);
      debugAcceptedBoxes_.emplace_back(candidate.box.get444from422(), candidate.numberEdgePoints);
      // projected position of the bottom center of the candidate box (start of the robot's feet)
      const std::optional<Vector2f> robotPosition = cameraMatrix_->pixelToRobot(
          candidate.box.bottomRight - Vector2i(candidate.box.size().x() / 2, 0));
      if (!robotPosition.has_value())
      {
        Log<M_VISION>(LogLevel::WARNING)
            << "RobotDetection: Projection to robot coordinates failed";
        continue;
      }
      // add half of the robot diameter to the vector in the vectors direction to get the robot's
      // center over ground
      const Vector2f robotCenterPosition =
          robotPosition.value() + robotPosition->normalized() * fieldDimensions_->robotDiameter / 2;
      robotData_->positions.push_back(robotCenterPosition);
    }
    else
    {
      debugRejectedBoxes_.emplace_back(candidate.box.get444from422(), candidate.numberEdgePoints);
    }
  }
}

void RobotDetection::sendRobotPositionImageForDebug()
{
  auto mountSeeds = mount_ + "." + imageData_->identification + "_position";
  if (!debug().isSubscribed(mountSeeds))
  {
    return;
  }
  Image image(imageData_->image422.to444Image());
  if (drawEdgePoints_())
  {
    for (const auto& column : columns_)
    {
      for (const auto& edge : column.edgePointsY)
      {
        image.drawCircle(Image422::get444From422Vector(Vector2i(column.x(), edge)), 2,
                         Color::ORANGE);
      }
    }
  }
  if (drawSeeds_())
  {
    for (const auto& column : columns_)
    {
      image.drawCircle(Image422::get444From422Vector(column.seedPosition), 2, Color::BLACK);
    }
  }
  if (drawWindows_())
  {
    for (const auto& window : debugWindows_)
    {
      image.drawRectangle(window.get444from422(), Color::YELLOW);
    }
  }
  if (drawRejectedCandidates_())
  {
    for (const auto& pair : debugRejectedBoxes_)
    {
      image.drawText(std::to_string(pair.second), pair.first.topLeft, Color::WHITE);
      image.drawRectangle(pair.first, Color::WHITE);
    }
  }
  if (drawCutCandidates_())
  {
    for (const auto& pair : debugCutBoxes_)
    {
      image.drawText(std::to_string(pair.second), pair.first.topLeft, Color::WHITE);
      image.drawRectangle(pair.first, Color::RED);
    }
  }
  if (drawAcceptedCandidates_())
  {
    for (const auto& pair : debugAcceptedBoxes_)
    {
      image.drawText(std::to_string(pair.second), pair.first.topLeft, Color::BLUE);
      image.drawRectangle(pair.first, Color::BLUE);
    }
  }
  for (const auto& position : robotData_->positions)
  {
    const std::optional<Vector2i> pixelRobotPosition = cameraMatrix_->robotToPixel(position);
    if (!pixelRobotPosition.has_value())
    {
      Log<M_VISION>(LogLevel::WARNING) << "RobotDetection: Projection to pixel coordinates failed";
      continue;
    }
    image.drawCircle(Image422::get444From422Vector(pixelRobotPosition.value()), 6, Color::PINK);
    image.drawCircle(Image422::get444From422Vector(pixelRobotPosition.value()), 5, Color::PINK);
    image.drawCross(Image422::get444From422Vector(pixelRobotPosition.value()), 20, Color::PINK);
  }
  // draw field border
  VecVector2i allBorderPoints = fieldBorder_->getBorderPoints();
  for (const auto& bp : allBorderPoints)
  {
    image[Image422::get444From422Vector(bp)] = Color::RED;
  }
  debug().sendImage(mountSeeds, image);
}

void RobotDetection::sendHistogramImageForDebug()
{
  auto mountHistogram = mount_ + "." + imageData_->identification + "_histogram";
  if (!debug().isSubscribed(mountHistogram))
  {
    return;
  }
  Image image(imageData_->image422.to444Image());
  std::vector<int> noFieldColorCounts(imageData_->image422.size.x());
  for (const auto& column : columns_)
  {
    noFieldColorCounts[column.seedPosition.x()] = column.edgePointsY.size();
    image.drawCircle(Image422::get444From422Vector(column.seedPosition), 2, Color::BLACK);
  }
  image.drawHistogram(noFieldColorCounts, Color::BLUE, 1);
  debug().sendImage(mountHistogram, image);
}
