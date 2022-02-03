#include "Vision/FilteredSegmentsProvider/FilteredSegmentsProvider.hpp"
#include "Tools/Chronometer.hpp"
#include "Tools/Storage/Color.hpp"

FilteredSegmentsProvider::FilteredSegmentsProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , fieldBorder_(*this)
  , imageData_(*this)
  , imageSegments_(*this)
  , drawVerticalScanlines_(*this, "drawVerticalScanlines", [] {})
  , drawVerticalEdges_(*this, "drawVerticalEdges", [] {})
  , drawHorizontalScanlines_(*this, "drawHorizontalScanlines", [] {})
  , drawHorizontalEdges_(*this, "drawHorizontalEdges", [] {})
  , filteredSegments_(*this)
{
}

void FilteredSegmentsProvider::cycle()
{
  {
    Chronometer time(debug(), mount_ + ".cycle_time." + imageData_->identification);
    gatherVerticalSegments();
    gatherHorizontalSegments();
  }
  sendDebug();
  filteredSegments_->valid = true;
}

void FilteredSegmentsProvider::gatherVerticalSegments()
{
  for (const auto& scanline : imageSegments_->verticalScanlines)
  {
    const auto firstSegmentInField = std::find_if(
        scanline.segments.begin(), scanline.segments.end(), [this](const auto& segment) {
          return fieldBorder_->isInsideField(segment.start) &&
                 fieldBorder_->isInsideField(segment.end);
        });
    for (auto it = firstSegmentInField; it != scanline.segments.end(); it++)
    {
      if (it->field < 0.5f)
      {
        filteredSegments_->vertical.emplace_back(&*it);
      }
    }
  }
}

void FilteredSegmentsProvider::gatherHorizontalSegments()
{
  for (const auto& scanline : imageSegments_->horizontalScanlines)
  {
    const auto firstSegmentInField = std::find_if(
        scanline.segments.begin(), scanline.segments.end(), [this](const auto& segment) {
          return fieldBorder_->isInsideField(segment.start) &&
                 fieldBorder_->isInsideField(segment.end);
        });
    for (auto it = firstSegmentInField; it != scanline.segments.end(); it++)
    {
      if (it->field < 0.5f)
      {
        if (!fieldBorder_->isInsideField(it->start) || !fieldBorder_->isInsideField(it->end))
        {
          break;
        }
        filteredSegments_->horizontal.emplace_back(&*it);
      }
    }
  }
}

// NOLINTNEXTLINE(readability-function-cognitive-complexity)
void FilteredSegmentsProvider::sendDebug() const
{
  const std::string mount = mount_ + "." + imageData_->identification;
  if (debug().isSubscribed(mount))
  {
    Image image(Image422::get444From422Vector(imageData_->image422.size), Color::BLACK);
    for (const auto& segment : filteredSegments_->vertical)
    {
      if (drawVerticalScanlines_())
      {

        image.drawLine(Image422::get444From422Vector(segment->start),
                       Image422::get444From422Vector(segment->end), Color{segment->ycbcr422});
        if (drawVerticalEdges_())
        {
          const auto edgeColor = segment->startEdgeType == EdgeType::RISING    ? Color::RED
                                 : segment->startEdgeType == EdgeType::FALLING ? Color::GREEN
                                                                               : Color::ORANGE;
          image.drawLine(Image422::get444From422Vector(segment->start),
                         Image422::get444From422Vector(segment->start) + Vector2i(2, 0), edgeColor);
          image.drawLine(Image422::get444From422Vector(segment->end),
                         Image422::get444From422Vector(segment->end) + Vector2i(2, 0), edgeColor);
        }
      }
    }
    for (const auto& segment : filteredSegments_->horizontal)
    {
      if (drawHorizontalScanlines_())
      {

        image.drawLine(Image422::get444From422Vector(segment->start),
                       Image422::get444From422Vector(segment->end), Color{segment->ycbcr422});
        if (drawHorizontalEdges_())
        {
          const auto edgeColor = segment->startEdgeType == EdgeType::RISING    ? Color::RED
                                 : segment->startEdgeType == EdgeType::FALLING ? Color::GREEN
                                                                               : Color::ORANGE;
          image.drawLine(Image422::get444From422Vector(segment->start),
                         Image422::get444From422Vector(segment->start) + Vector2i(0, 2), edgeColor);
          image.drawLine(Image422::get444From422Vector(segment->end),
                         Image422::get444From422Vector(segment->end) + Vector2i(0, 2), edgeColor);
        }
      }
    }
    debug().sendImage(mount, image);
  }
}
