#ifndef FIELDCOLOR_HPP_
#define FIELDCOLOR_HPP_

#include <cmath>

#include "Framework/DataType.hpp"
#include "Tools/Storage/Image.hpp"

class FieldColor : public DataType<FieldColor> {
public:
  /**
   * @brief Field color threshold for the Y channel
   *
   * @author Georg Felbinger
   */
  int thresholdY;
  /**
   * @brief Field color threshold for u and v channel
   *
   * @author Georg Felbinger
   */
  int thresholdUvSquared;
  int meanCb;
  int meanCr;
  /// whether the field color is valid
  bool valid;

  /**
   * @brief Return if a pixel falls within the field color range
   * A pixel is green, if the length of the (u,v) vector is lower than the specified threshold
   * The conversion to YUV is done by U=0.872021*Cb, V=1.229951*Cr
   * Since the values are squared, the U-Factor gets 0.75 and the V-Factor gets 1.5.
   * With this knowledge, we can multiply the equation U^2 + V^2 < t by 2, so the multiplications get Bitshifts.
   * The Y channel is threshed, since Cb and Cr are kind of random on high Y.
   * @return true if pixel has field color, false if not
   *
   * @author Georg Felbinger
   */
  bool isFieldColor(const Color &pixel) const
  {
    const int cb = (pixel.cb_ - meanCb);
    const int cr = (pixel.cr_ - meanCr);
    return pixel.y_ < thresholdY && cb * cb + cr * cr * 2 < thresholdUvSquared;
  }

  /**
   * @brief reset sets the field color to a defined state
   */
  void reset()
  {
    valid = false;
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["thresholdY"] << thresholdY;
    value["thresholdUvSquared"] << thresholdUvSquared;
    value["valid"] << valid;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["thresholdY"] >> thresholdY;
    value["thresholdUvSquared"] >> thresholdUvSquared;
    value["valid"] >> valid;
  }
};

#endif
