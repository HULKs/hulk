#include <iterator>

#include "ImageSegmenter.hpp"
#include "Utils/Algorithms.hpp"
#include "print.hpp"

#include "Tools/Chronometer.hpp"
#include "Tools/Storage/Image.hpp"

ImageSegmenter::ImageSegmenter(const ModuleManagerInterface& manager)
  : Module(manager, "ImageSegmenter")
  , draw_full_image_(*this, "draw_full_image", [] {})
  , edge_threshold_(*this, "edge_threshold", [] {})
  , num_scanlines_(*this, "num_scanlines", [] {})
  , image_data_(*this)
  , camera_matrix_(*this)
  , field_color_(*this)
  , robot_projection_(*this)
  , image_regions_(*this)
{
}

void ImageSegmenter::haveEdge(int y, Scanline& scanline, EdgeType type)
{
  Region& region = scanline.regions.back();
  region.end = y;
  region.end_edge = type;
  // TODO: determine color in another way for some region lengths
  int diff = region.end - region.start;
  if (diff >= 6)
  {
    int spacing = diff / 6;
    Color c1 = image_data_->image.at(region.start + spacing, scanline.x);
    Color c2 = image_data_->image.at(region.start + spacing * 2, scanline.x);
    Color c3 = image_data_->image.at(region.start + spacing * 3, scanline.x);
    Color c4 = image_data_->image.at(region.start + spacing * 4, scanline.x);
    Color c5 = image_data_->image.at(region.start + spacing * 5, scanline.x);
    region.color = Color(Algorithms::median(c1.y_, c2.y_, c3.y_, c4.y_, c5.y_), Algorithms::median(c1.cb_, c2.cb_, c3.cb_, c4.cb_, c5.cb_),
                         Algorithms::median(c1.cr_, c2.cr_, c3.cr_, c4.cr_, c5.cr_));
  }
  else if (diff > 2)
  {
    Color c1 = image_data_->image.at(region.start, scanline.x);
    Color c2 = image_data_->image.at((region.start + region.end) >> 1, scanline.x);
    Color c3 = image_data_->image.at(region.end, scanline.x);
    region.color = Color(Algorithms::median(c1.y_, c2.y_, c3.y_), Algorithms::median(c1.cb_, c2.cb_, c3.cb_), Algorithms::median(c1.cr_, c2.cr_, c3.cr_));
  }
  else
  {
    region.color = image_data_->image.at((region.start + region.end) >> 1, scanline.x);
  }
  region.field = field_color_->isFieldColor(region.color) * 1.0f;
  if (type != EdgeType::BORDER)
  {
    Region next;
    next.start = y;
    next.start_edge = type;
    scanline.regions.push_back(next);
  }
}

void ImageSegmenter::createScanlines()
{
  Scanline scanline;
  Region region;
  ScanlineState scanline_state;
  std::vector<ScanlineState> scanline_states;
  const int scanline_spacing = image_data_->image.size_.x() / num_scanlines_();
  const int horizon = std::min(std::min(camera_matrix_->getHorizonHeight(0), camera_matrix_->getHorizonHeight(image_data_->image.size_.x() - 1)),
                               image_data_->image.size_.y() - 1);
  image_regions_->scanlines.reserve(num_scanlines_());
  region.start = horizon;
  region.start_edge = EdgeType::BORDER;
  scanline.id = 1;
  scanline.x = scanline_spacing / 2;
  scanline.regions.push_back(region);
  scanline_state.g_min = edge_threshold_();
  scanline_state.g_max = -edge_threshold_();
  scanline_state.y_peak = 0;
  scanline_states.reserve(num_scanlines_());

  for (int i = 0; i < num_scanlines_(); i++)
  {
    scanline.y_max = image_data_->image.size_.y() - 1;
    for (const auto& line : robot_projection_->lines)
    {
      auto x_min = std::min(line.p1.x(), line.p2.x());
      auto x_max = std::max(line.p1.x(), line.p2.x());
      if (x_min <= scanline.x && x_max >= scanline.x)
      {
        if (line.p1.x() == line.p2.x())
        {
          scanline.y_max = std::max(0, std::min(std::min(line.p1.y(), line.p2.y()), scanline.y_max));
        }
        else
        {
          scanline.y_max = std::max(0, std::min(line.getY(scanline.x), scanline.y_max));
        }
      }
    }
    image_regions_->scanlines.push_back(scanline);
    scanline_state.last = image_data_->image.at(horizon, scanline.x);
    scanline_state.scanline = &(image_regions_->scanlines.back());
    scanline_states.push_back(scanline_state);
    scanline.id++;
    scanline.x += scanline_spacing;
  }

  for (int y = horizon + 2; y < image_data_->image.size_.y(); y += 2)
  {
    for (auto& it : scanline_states)
    {
      if (y > it.scanline->y_max)
      {
        continue;
      }

      Color color = image_data_->image.at(y, it.scanline->x);
      int diff = color.y_ - it.last.y_;
      if (diff > it.g_max)
      {
        if (it.g_min < -edge_threshold_())
        {
          haveEdge(it.y_peak, (*it.scanline), EdgeType::FALLING);
        }
        it.g_max = diff;
        it.g_min = edge_threshold_();
        it.y_peak = y - 1;
      }
      if (diff < it.g_min)
      {
        if (it.g_max > edge_threshold_())
        {
          haveEdge(it.y_peak, (*it.scanline), EdgeType::RISING);
        }
        it.g_min = diff;
        it.g_max = -edge_threshold_();
        it.y_peak = y - 1;
      }
      it.last = color;
    }
  }
  for (auto& it : image_regions_->scanlines)
  {
    haveEdge(it.y_max, it, EdgeType::BORDER);
  }
}

void ImageSegmenter::sendImageForDebug(const Image& image)
{
  if (!debug().isSubscribed(mount_ + "." + image_data_->identification + "_image"))
  {
    return;
  }

  Image regionImage(image.size_);
  for (int i = 0; i < image.size_.x() * image.size_.y(); i++)
  {
    regionImage.data_[i] = Color::BLACK;
  }
  if (draw_full_image_() && !image_regions_->scanlines.empty())
  {
    for (auto scanline = image_regions_->scanlines.begin(); std::next(scanline) != image_regions_->scanlines.end(); ++scanline)
    {
      for (auto& region : scanline->regions)
      {
        for (int i = 0; i < (std::next(scanline)->x - scanline->x); ++i)
        {
          regionImage.line(Vector2i(scanline->x + i, region.start), Vector2i(scanline->x + i, region.end - 1), region.color);
        }
      }
    }
  }
  else
  {
    for (auto& it : image_regions_->scanlines)
    {
      for (auto& it2 : it.regions)
      {
        regionImage.line(Vector2i(it.x, it2.start), Vector2i(it.x, it2.end - 1), it2.color);
        regionImage.line(Vector2i(it.x + 1, it2.start), Vector2i(it.x + 1, it2.end - 1), it2.color);
      }
    }
  }
  debug().sendImage(mount_ + "." + image_data_->identification + "_image", regionImage);
}

void ImageSegmenter::cycle()
{
  {
    Chronometer time(debug(), mount_ + ".cycle_time");
    createScanlines();
    image_regions_->valid = true;
  }
  sendImageForDebug(image_data_->image);
}
