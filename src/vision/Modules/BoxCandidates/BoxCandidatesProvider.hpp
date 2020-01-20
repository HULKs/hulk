#pragma once

#include "Data/BoxCandidates.hpp"
#include "Data/CameraMatrix.hpp"
#include "Data/FieldBorder.hpp"
#include "Data/FieldColor.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/ImageData.hpp"
#include "Data/ImageSegments.hpp"
#include "Data/IntegralImageData.hpp"
#include "Data/RobotProjection.hpp"
#include "Framework/Module.hpp"

class Brain;

/*
 * @brief Generates candidates for ball detection
 * This module searches for bright spots of projected ball size in one channel of the image. The
 * image is divided into several 'blocks'. One 'block' contains blockSize_ * blockSize_ pixels.
 * These blocks are introduced to reduce computational load and necessary storage for candidates by
 * generating an upper bound of possible candidates. For each 'block' the pixel position with the
 * highest 'rating' is searched. To save computation the step size searching for the maximum is
 * increased for large projected ball sizes. This can be adjusted by changing stepsPerBallSize_. A
 * 'rating' of a pixel is determined by comparing the sums of two slightly differently sized boxes
 * to find a high concentration of pixel values of ball size. The best position of each block is
 * saved and evaluated to be higher rated than minBoxRating_. Afterwards the remaining candidates
 * are sorted by rating and the modules produces maxCandidateNumber_ number of candidates.
 */
class BoxCandidatesProvider : public Module<BoxCandidatesProvider, Brain>
{
public:
  /// the name of this module
  ModuleName name = "BoxCandidatesProvider";
  /**
   * @brief BoxCandidatesProvider initializes members
   * @param manager a reference to brain
   */
  BoxCandidatesProvider(const ModuleManagerInterface& manager);

  /**
   * @brief cycle tries to find a ball
   */
  void cycle() override;

private:
  /// structure combining a position with a radius and a rating
  struct CandidateBox
  {
    CandidateBox() = default;
    CandidateBox(const int& rating, Vector2i pos, const int& boxRadius)
      : rating(rating)
      , pos(std::move(pos))
      , boxRadius(boxRadius)
    {
    }
    int rating{std::numeric_limits<int>::min()};
    Vector2i pos{0, 0};
    int boxRadius{0};
  };

  /// to project the ballsize into the image
  const Dependency<CameraMatrix> cameraMatrix_;
  /// current image to find the ball
  const Dependency<ImageData> imageData_;
  /// generated integral image
  const Dependency<IntegralImageData> integralImageData_;
  /// all candidates below fieldBorder will be rejected
  const Dependency<FieldBorder> fieldBorder_;
  /// for checking whether a pixel has fieldColor
  const Dependency<FieldColor> fieldColor_;
  /// contains the ballSize
  const Dependency<FieldDimensions> fieldDimensions_;
  /// to check whether a candidate is on the own robot
  const Dependency<RobotProjection> robotProjection_;


  /// the size in pixel of candidate grouping block
  const ConditionalParameter<int> blockSize_;
  /// the minimum Y channel value of a pixel to be "bright"
  const ConditionalParameter<int> brightPixelThreshold_;
  /// the maximum Y channel value of a pixel to be "dark"
  const ConditionalParameter<int> darkPixelThreshold_;
  /// factor multiplied with the ball radius to get the inner detection box
  const ConditionalParameter<float> innerRadiusScale_;
  /// factor multiplied with the ball radius to get the larger detection box
  const ConditionalParameter<float> outerRadiusScale_;
  /// maximum number of candidates sampled
  const ConditionalParameter<int> maxCandidateNumber_;
  /// factor multiplied with the candidate radius to determine if its the same candidate
  const ConditionalParameter<float> mergeToleranceFactor_;
  /// minimum rating of a block to be a candidate
  const ConditionalParameter<int> minBoxRating_;
  /// minimum radius to be a candidate (skips detection of regions)
  const ConditionalParameter<int> minPixelRadius_;
  /// minimum number of bright pixels in sample image to be a valid candidate
  const ConditionalParameter<int> numberBrightPixels_;
  /// minimum number of dark pixels in sample image to be a valid candidate
  const ConditionalParameter<int> numberDarkPixels_;
  /// the maximum number of fieldColorPixels a candidate could have
  const ConditionalParameter<int> maxNumberFieldPixels_;
  /// the size in pixel of one sample
  const Parameter<int> sampleSize_;
  /// whether blocks outside the field should be skipped
  const ConditionalParameter<bool> skipOutsideField_;
  /// number of steps calculated a rating for per ball size
  const ConditionalParameter<int> stepsPerBallSize_;

  /**
   * Calculates the rating of all pixels concerning the stepsPerBallSize_ in the given block and
   * saves the best result in parameter output.
   * @param blockY the start Y coordinate of the block
   * @param blockX the start X coordinate of the block
   * @param output reference to the CandidateBox where the best result will be saved
   */
  void calculateBlockRating(int& blockY, int& blockX, CandidateBox& output) const;
  /**
   * Calculates the best rating for all boxes and stores boxes with a rating greater than
   * minBoxRating_ in parameter candidates.
   * @param candidates reference to the vector of all candidate boxes which fulfill the condition
   */
  void findCandidateBoxes(std::vector<CandidateBox>& candidates);
  /**
   * Returns the rating of a point in the integral image. The rating is calculated by a box with
   * size of innerRadius and a larger box of size outerRadius. The sum of pixel values of each box
   * is divided by its respective area and the ratio yields the rating.
   * @param integralX The X coordinate of the point
   * @param integralY The Y coordinate of the point
   * @param innerRadius The radius of the inner box
   * @param outerRadius The radius of the outer box
   * @return The rating of the point at (integralX, integralY)
   */
  int getRating(int& integralX, int& integralY, const int& innerRadius,
                const int& outerRadius) const;
  /**
   * Sorts all candidate boxes given by the parameter candidates by their ratings and takes the
   * results with highest rating until the number of maxCandidateNumber_ is reached. A candidate
   * cannot be added if a candidate within proximity is already in the result list.
   * @param candidates All candidate boxes to be sorted and filtered
   * @return The vector of Circles which fulfill the conditions and serve as candidate
   */
  std::vector<Circle<int>> getBestCandidates(std::vector<CandidateBox>& candidates) const;
  /**
   * Determines whether a candidate position is inside one of the candidate circles of the given
   * list.
   * @param pos The position of the candidate
   * @param circles All candidate circles which should be checked
   * @return Whether pos is inside one of the candate circles
   */
  bool isInsideCandidate(const Vector2i& pos, const std::vector<Circle<int>>& circles) const;

  /**
   * Samples the image representing the given candidate. This will serve as input for the neural
   * network.
   * @param circle The candidate circle representing the position and radius of the candidate
   * @param sampleSize The size of the image to be sampled
   * @param colorSampled A reference to the sampled image vector
   * @return whether the checks on the sample are all fulfilled
   */
  bool sampleBoundingBox(const Circle<int>& circle, unsigned int sampleSize,
                         std::vector<std::uint8_t>& colorSampled);

  /*
   * @brief send debug information
   */
  void sendDebug() const;

  // the generated box candidates
  Production<BoxCandidates> boxCandidates_;
};
