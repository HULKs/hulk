#pragma once

#include "Data/FieldBorder.hpp"
#include "Data/FilteredSegments.hpp"
#include "Data/ImageData.hpp"
#include "Framework/Module.hpp"

class Brain;

/**
 * @brief Provides FilteredSegments
 */
class FilteredSegmentsProvider : public Module<FilteredSegmentsProvider, Brain>
{
public:
  /// the name of this module
  ModuleName name__{"FilteredSegmentsProvider"};

  /**
   *@brief The constructor of this class
   */
  explicit FilteredSegmentsProvider(const ModuleManagerInterface& manager);

  void cycle() override;

private:
  const Dependency<FieldBorder> fieldBorder_;
  const Dependency<ImageData> imageData_;
  const Dependency<ImageSegments> imageSegments_;

  /// whether vertical scanlines should be shown in debug images
  const Parameter<bool> drawVerticalScanlines_;
  /// whether vertical edges should be shown in debug images
  const Parameter<bool> drawVerticalEdges_;
  /// whether horizontal scanlines should be shown in debug images
  const Parameter<bool> drawHorizontalScanlines_;
  /// whether horizontal edges should be shown in debug images
  const Parameter<bool> drawHorizontalEdges_;

  /// the segments that are below the field border and no field
  Production<FilteredSegments> filteredSegments_;

  /**
   * @brief iterates all vertical segments and filters segments with field color and outside the
   * field
   */
  void gatherVerticalSegments();
  /**
   * @brief iterates all horizontal segments and filters segments with field color and outside the
   * field
   */
  void gatherHorizontalSegments();
  /**
   * @brief sends debug image
   */
  void sendDebug() const;
};
