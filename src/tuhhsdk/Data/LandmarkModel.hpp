#pragma once

#include <vector>

#include "Framework/DataType.hpp"
#include "Tools/Math/Eigen.hpp"

class LandmarkModel : public DataType<LandmarkModel> {
public:
  /**
frief Goal stores to posts at once
   */
  struct Goal : public Uni::To, public Uni::From {
    /**
     * @brief Goal constructs a Goal from two posts
     */
    Goal()
    {
      left = {0, 0};
      right = {0, 0};
    }

    /**
     * @brief Goal constructs a Goal from two posts
     * @param left the left post of the goal
     * @param right the right post of the goal
     */
    Goal(const Vector2f& left, const Vector2f& right) :
      left(left),
      right(right)
    {
    }
    /// relative position of the left post
    Vector2f left;
    /// relative position of the right post
    Vector2f right;

    /**
   * @see function in DataType
   */
    virtual void toValue(Uni::Value& value) const
    {
      value = Uni::Value(Uni::ValueType::OBJECT);
      value["left"] << left;
      value["right"] << right;
    }

    /**
     * @see function in DataType
     */
    virtual void fromValue(const Uni::Value& value)
    {
      value["left"] >> left;
      value["right"] >> right;
    }
  };
  /// a vector of complete goals
  std::vector<Goal> goals;
  /**
   * @brief reset clears the goal vector
   */
  void reset()
  {
    goals.clear();
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["goals"] << goals;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["goals"] >> goals;
  }
};
