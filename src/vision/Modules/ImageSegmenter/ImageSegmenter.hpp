#pragma once

#include "Framework/Module.hpp"

#include "Data/CameraMatrix.hpp"
#include "Data/FieldColor.hpp"
#include "Data/ImageData.hpp"
#include "Data/ImageSegments.hpp"
#include "Data/RobotProjection.hpp"
#include <Modules/Projection/ProjectionCamera.hpp>
#include <Tools/Kinematics/ForwardKinematics.h>
#include <set>


class Brain;

class ImageSegmenter : public Module<ImageSegmenter, Brain>
{
public:
  /// the name of this module
  ModuleName name = "ImageSegmenter";
  /**
   * ImageSegmenter constructor
   * @param manager a reference to the brain object
   */
  ImageSegmenter(const ModuleManagerInterface& manager);
  void cycle();

private:
  struct ScanlineStateVertical
  {
    // edge detection state
    int gMin;
    // edge detection state
    int gMax;
    // the y coordinate where the edge intensity was highest
    int yPeak;
    // the number of sampled points within the last segment
    int scanPoints;
    // the previous y value on the scanline
    std::uint8_t lastYValue;
    // the scanline this state belongs to
    VerticalScanline* scanline;
  };

  struct ScanlineStateHorizontal
  {
    // edge detection states
    int gMin;
    int gMax;
    // the x coordinate whre the edge intensity was highest
    int xPeak;
    // the number of sampled points within the last segment
    int scanPoints;
    // the previous color on the scanline
    const YCbCr422* lastYCbCr422;

    void reset(const int edgeThreshold, const YCbCr422* ycbcr422)
    {
      gMin = edgeThreshold;
      gMax = -edgeThreshold;
      xPeak = 0;
      scanPoints = 1;
      lastYCbCr422 = ycbcr422;
    }
  };

  /**
   * @brief median computes the median of five elements
   * http://stackoverflow.com/questions/480960/code-to-calculate-median-of-five-in-c-sharp/2117018#2117018
   * @param a
   * @param b
   * @param c
   * @param d
   * @param e
   */
  uint8_t median(uint8_t a, uint8_t b, uint8_t c, uint8_t d, uint8_t e);

  /**
   * @brief median computes the median of three elements
   * http://stackoverflow.com/questions/480960/code-to-calculate-median-of-five-in-c-sharp/2117018#2117018
   * @param a
   * @param b
   * @param c
   */
  uint8_t median(uint8_t a, uint8_t b, uint8_t c);

  /**
   * @brief addSegment is a handler for edges that manages segment creation
   * @param peak the coordinate at which the edge has been found
   * @param scanline the scanline on which the edge has been found
   * @param type whether this is a falling, rising, robot or image border edge
   * @param scanPoints the number of sampled points within this segment
   */
  void addSegment(const Vector2i& peak, Scanline& scanline, EdgeType type, int scanPoints);

  /**
   * @brief isOnRobot checks whether a pixel is on himself
   */
  bool isOnRobot(const Vector2i& pos);
  /**
   * @brief createVerticalScanlines scans the image on vertical scanlines and creates segments of
   * similar color
   * @tparam useMedian if true the median of the pixel's y value and the y values of the pixel
   * above and below is evaluated for segmentation instead of simply the pixel's y value
   */
  template <bool useMedian>
  void createVerticalScanlines();
  /**
   * @brief createHorizontalScanlines scans the image on horizontal scanlines and creates segments
   * of similar color
   */
  void createHorizontalScanlines();
  /// Calculates the lookup tables {@link scanGrids_}.
  void calculateScanGrids();
  /**
   * @brief sendDebug
   * @param image the camera image in which to draw the segments
   */
  void sendDebug();

  bool isRobotCheckNecessary(const int y) const;

  /**
   * @brief Prevents race condition of in-cycle change of the number of scanlines
   */
  bool updateScanlines_;
  /// @brief whether the scangrid for a camera is valid
  std::array<bool, 2> scanGridsValid_;

  const Parameter<bool> drawFullImage_;
  const Parameter<std::array<int, 2>> edgeThresholdHorizontal_;
  const Parameter<std::array<int, 2>> edgeThresholdVertical_;
  const Parameter<int> numScanlines_;
  const Parameter<bool> drawEdges_;
  const Parameter<bool> useMedianVerticalTop_;
  const Parameter<bool> useMedianVerticalBottom_;

  const Dependency<ImageData> imageData_;
  const Dependency<CameraMatrix> cameraMatrix_;
  const Dependency<FieldColor> fieldColor_;
  const Dependency<RobotProjection> robotProjection_;

  Production<ImageSegments> imageSegments_;
};
