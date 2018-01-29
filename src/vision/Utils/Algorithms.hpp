#pragma once

#include "print.hpp"
#include <vector>
// This overloads abs for floating point types. No fabs needed.
#include <cmath>
#include <limits>

#include "Tools/Math/Circle.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Line.hpp"
#include "Tools/Math/Random.hpp"

namespace Algorithms
{
  /**
   * calculate the intersection of two (infinite) lines
   * @param l1 the first line
   * @param l2 the second line
   * @return a vector describing the intersection
   */
  template <typename T>
  Vector2<T> getIntersection(const Line<T>& l1, const Line<T>& l2)
  {
    Vector2<T> result;
    T denominator = (l2.p2.y() - l2.p1.y()) * (l1.p2.x() - l1.p1.x()) - (l1.p2.y() - l1.p1.y()) * (l2.p2.x() - l2.p1.x());
    if (!denominator)
    {
      result = Vector2<T>(-1, -1);
    }
    else
    {
      result.x() = ((l2.p2.x() - l2.p1.x()) * (l1.p2.x() * l1.p1.y() - l1.p1.x() * l1.p2.y()) -
                    (l1.p2.x() - l1.p1.x()) * (l2.p2.x() * l2.p1.y() - l2.p1.x() * l2.p2.y())) /
                   denominator;
      result.y() = ((l1.p1.y() - l1.p2.y()) * (l2.p2.x() * l2.p1.y() - l2.p1.x() * l2.p2.y()) -
                    (l2.p1.y() - l2.p2.y()) * (l1.p2.x() * l1.p1.y() - l1.p1.x() * l1.p2.y())) /
                   denominator;
    }
    return result;
  }

  /**
   * calculate the minimal distance of a point to a line
   * @param l the line
   * @param p vector whose distance shall be calculated
   * @return the distance
   */
  template <typename T>
  T getSquaredLineDistance(const Line<T>& l, const Vector2<T>& p)
  {
    T nominator = (l.p2.y() - l.p1.y()) * p.x() - (l.p2.x() - l.p1.x()) * p.y() + l.p2.x() * l.p1.y() - l.p2.y() * l.p1.x();
    T denominator = (l.p2 - l.p1).squaredNorm();
    return (nominator * nominator) / denominator;
  }

  /**
   * calculate the minimal distance of a point to a circle
   * @param circle the circle
   * @param p vector whose distance shall be calculated
   * @return the distance
   */
  template <typename T>
  T getCircleDistance(const Circle<T>& circle, const Vector2<T>& p)
  {
    return std::abs(std::sqrt((p.x() - circle.center.x()) * (p.x() - circle.center.x()) + (p.y() - circle.center.y()) * (p.y() - circle.center.y())) -
                    circle.radius);
  }

  /**
   * do ransac for line
   * @param points vector of input points
   * @param best is filled with the best matching line points
   * @param unused is filled with points outside the line
   * @param iterations number of iterations that should be executed
   * @param max_distance radius in which points are accepted
   */
  template <typename T>
  bool ransacLine(Line<T>& bestLine, const VecVector2<T>& points, VecVector2<T>& best, VecVector2<T>& unused, unsigned int iterations, T max_distance)
  {
    bool valid = false;
    Line<T> line;
    T distance;
    const T sqr_max_distance = max_distance * max_distance;
    // Keep a buffer and a back-buffer, if we found the best line,
    // just swap the buffer and add to the other, until we find a better one.
    VecVector2<T> current_used_1, current_unused_1;
    VecVector2<T> current_used_2, current_unused_2;

    VecVector2<T>* current_used = &current_used_1;
    VecVector2<T>* current_unused = &current_unused_1;

    unsigned int max_score = 0;
    if (points.size() < 2)
    {
      best.clear();
      unused = points;
      return valid;
    }

    current_used_1.reserve(points.size());
    current_used_2.reserve(points.size());
    current_unused_1.reserve(points.size());
    current_unused_2.reserve(points.size());

    for (unsigned int i = 0; i < iterations; i++)
    {
      line.p1 = points[Random::uniformInt(0, points.size() - 1)];
      line.p2 = points[Random::uniformInt(0, points.size() - 1)];

      if (line.p1 == line.p2)
      {
        continue;
      }
      current_used->clear();
      current_unused->clear();

      for (auto& point : points)
      {
        distance = getSquaredLineDistance(line, point);
        if (distance <= sqr_max_distance)
        {
          current_used->push_back(point);
        }
        else
        {
          current_unused->push_back(point);
        }
      }

      if (current_used->size() > max_score)
      {
        max_score = current_used->size();
        bestLine = line;
        if (current_used == &current_used_1)
        {
          current_used = &current_used_2;
          current_unused = &current_unused_2;
        }
        else
        {
          current_used = &current_used_1;
          current_unused = &current_unused_1;
        }
      }
    }

    best = current_used == &current_used_1 ? current_used_2 : current_used_1;
    unused = current_unused == &current_unused_1 ? current_unused_2 : current_unused_1;

    if (best.empty() || bestLine.p1 == bestLine.p2)
    {
      best.clear();
      unused = points;
      return valid;
    }

    valid = true;
    return valid;
  }

  /**
   * @brief calculateCenterPoint
   * This function calculates a center Point from a circle which is formed
   * by the three points.
   * @param p1
   * Point 1 of the circle
   * @param p2
   * Point 2 of the circle
   * @param p3
   * Point 3 of the circle
   * @return
   * The Center of the circle
   */
  template <typename T>
  Vector2<T> calculateCenterPoint(const Vector2<T>& p1, const Vector2<T>& p2, const Vector2<T>& p3)
  {
    Vector2<T> result;
    float m_a, m_b;
    if ((p1.x() == p2.x()) || (p2.x() == p3.x()))
    {
      return result;
    }
    m_a = (p2.y() - p1.y()) / (p2.x() - p1.x());
    m_b = (p3.y() - p2.y()) / (p3.x() - p2.x());
    if (!m_a || (m_b == m_a))
    {
      return result;
    }
    result.x() = (m_a * m_b * (p1.y() - p3.y()) + m_b * (p1.x() + p2.x()) - m_a * (p2.x() + p3.x())) / (2 * (m_b - m_a));
    result.y() = -(1 / m_a) * (result.x() - (p1.x() + p2.x()) / 2) + (p1.y() + p2.y()) / 2;
    return result;
  }

  /**
   * do ransac for circle fitting
   * @param points vector of input points
   * @param best is filled with the best matching circle
   * @param unused is filled with points outside the circle
   * @param iterations number of iterations that should be executed
   * @param max_distance radius in which points are accepted
   * @param circlescore the score that the circle yielded
   * @param radius can specify a specific radius for the fitted circle, 0 otherwise
   */
  template <typename T>
  Circle<T> ransacCircle(const VecVector2<T>& points, VecVector2<T>& best, VecVector2<T>& unused, unsigned int iterations, T max_distance, int& circlescore,
                         T radius = 0, float minRadius = 0, float maxRadius = 0)
  {
    Circle<T> result, current_circle;
    T distance, min_error = std::numeric_limits<T>::max();
    VecVector2<T> current_best, current_unused;
    Vector2<T> p1, p2, p3;
    unsigned int score, max_score = 0;
    if (points.size() < 3)
    {
      circlescore = 0;
      best.clear();
      unused = points;
      return result;
    }
    if (radius)
    {
      current_circle.radius = radius;
    }
    for (unsigned int i = 0; i < iterations; i++)
    {
      p1 = points[rand() % points.size()];
      p2 = points[rand() % points.size()];
      p3 = points[rand() % points.size()];
      if ((p1 == p2) || (p1 == p3) || (p2 == p3))
      {
        continue;
      }
      score = 0;
      current_best.clear();
      current_unused.clear();
      current_circle.center = calculateCenterPoint(p1, p2, p3);
      if (!radius)
      {
        current_circle.radius = (current_circle.center - p1).norm();

        // Filter by min and max radius:
        if (maxRadius)
        {
          if (current_circle.radius > maxRadius || current_circle.radius < minRadius)
          {
            continue;
          }
        }
      }
      T error = 0;
      for (auto it = points.begin(); it != points.end(); it++)
      {
        distance = getCircleDistance(current_circle, *it);
        if (distance <= max_distance)
        {
          error += distance;
          score++;
          current_best.push_back(*it);
        }
        else
        {
          current_unused.push_back(*it);
        }
      }
      if (score > max_score || (score == max_score && error < min_error))
      {
        max_score = score;
        min_error = error;
        best = current_best;
        unused = current_unused;
        result = current_circle;
      }
    }
    circlescore = max_score;
    return result;
  }

  /**
   * @brief median computes the median of five elements
   * http://stackoverflow.com/questions/480960/code-to-calculate-median-of-five-in-c-sharp/2117018#2117018
   * @param a
   * @param b
   * @param c
   * @param d
   * @param e
   */
  template <typename T>
  T median(T a, T b, T c, T d, T e)
  {
    return b < a ? d < c ? b < d ? a < e ? a < d ? e < d ? e : d : c < a ? c : a : e < d ? a < d ? a : d : c < e ? c : e
                                 : c < e ? b < c ? a < c ? a : c : e < b ? e : b : b < e ? a < e ? a : e : c < b ? c : b
                         : b < c ? a < e ? a < c ? e < c ? e : c : d < a ? d : a : e < c ? a < c ? a : c : d < e ? d : e
                                 : d < e ? b < d ? a < d ? a : d : e < b ? e : b : d < b ? d : b
                 : d < c ? a < d ? b < e ? b < d ? e < d ? e : d : c < b ? c : b : e < d ? b < d ? b : d : c < e ? c : e
                                 : c < e ? a < c ? b < c ? b : c : e < a ? e : a : a < e ? b < e ? b : e : c < a ? c : a
                         : a < c ? b < e ? b < c ? e < c ? e : c : d < b ? d : b : e < c ? b < c ? b : c : d < e ? d : e
                                 : d < e ? a < d ? b < d ? b : d : e < a ? e : a : a < e ? b < e ? b : e : d < a ? d : a;
  }

  template <typename T>
  T median(T a, T b, T c)
  {
    return a > b ? b > c ? b : a > c ? c : a : a > c ? a : b > c ? c : b;
  }
}
