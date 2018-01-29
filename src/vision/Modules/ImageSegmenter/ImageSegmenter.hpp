#pragma once

#include "Framework/Module.hpp"

#include "Data/CameraMatrix.hpp"
#include "Data/FieldColor.hpp"
#include "Data/ImageData.hpp"
#include "Data/ImageRegions.hpp"
#include "Data/RobotProjection.hpp"

class Brain;

class ImageSegmenter : public Module<ImageSegmenter, Brain>
{
public:
  /**
   * ImageSegmenter constructor
   * @param manager a reference to the brain object
   *
   * @author Erik Schröder and Pascal Loth
   */
  ImageSegmenter(const ModuleManagerInterface& manager);

  void cycle();

private:
  struct ScanlineState
  {
    /// edge detection state
    int g_min;
    /// edge detection state
    int g_max;
    /// the y coordinate where the edge intensity was highest
    int y_peak;
    /// the previous color on the scanline
    Color last;
    /// the scanline this state belongs to
    Scanline* scanline;
  };
  /**
   * @brief haveEdge is a handler for edges that manages region creation
   * @param y the y coordinate at which the edge has been found
   * @param scanline the scanline on which the edge has been found
   * @param type whether this is a falling or rising edge (or the image border)
   */
  void haveEdge(int y, Scanline& scanline, EdgeType type);
  /**
   * @brief createScanlines scans the image on vertical scanlines and creates regions of similar color
   */
  void createScanlines();
  /**
   * @brief sendImageForDebug
   * @param image the camera image in which to draw the region lines
   * @author Arne Hasselbring
   */
  void sendImageForDebug(const Image& image);
  /**
   * @brief Spacing between pixel sampling points
   *
   * Specifies the spacing between points of the subsampling grid used for the histogram.
   *
   * @author Erik Schröder and Pascal Loth
   */
  static const int GRID_SPACING = 4; // 16

  const Parameter<bool> draw_full_image_;
  const Parameter<int> edge_threshold_;
  const Parameter<int> num_scanlines_;
  const Dependency<ImageData> image_data_;
  const Dependency<CameraMatrix> camera_matrix_;
  const Dependency<FieldColor> field_color_;
  const Dependency<RobotProjection> robot_projection_;
  Production<ImageRegions> image_regions_;
};
