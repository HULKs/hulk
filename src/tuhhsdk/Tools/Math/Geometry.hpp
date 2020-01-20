#pragma once

#include "Circle.hpp"
#include "ConvexPolygon.hpp"
#include "Line.hpp"
#include "Plane.hpp"

namespace Geometry
{
  /**
   * Based on the 2nd answer at
   * https://math.stackexchange.com/questions/62633/orthogonal-projection-of-a-point-onto-a-line
   *
   * @brief getPointToLineVector calculates the orthogonal vector between a point and an infinitely
   * long line
   * @param line an infinite straight line
   * @param p a point
   * @return the shortest vector between the point and the line, pointing from point to line
   */
  inline Vector2f getPointToLineVector(const Line<float>& line, const Vector2f& p)
  {
    const Vector2f lineVec = line.p1 - line.p2;
    const Matrix2f projectionMat =
        (lineVec * lineVec.transpose()) / (lineVec.transpose() * lineVec);
    return (projectionMat * (p - line.p1) + line.p1) - p;
  }

  /**
   * @brief getAngleBetween calculates the angle between two direction vectors [0,pi]
   * @param directionVector1 the first direction vector
   * @param directionVector2 the second direction vector
   * @param[out] angle the calculated angle
   * @param returnSmallestAngle whether return the smallest angle [0,pi/2]
   * @return bool whether the calculation was successfull
   */
  inline bool getAngleBetween(const Vector2f& directionVector1, const Vector2f& directionVector2,
                              float& angle, bool returnSmallestAngle = true)
  {
    // calculate length of vectors
    float lenVec1 = directionVector1.norm();
    float lenVec2 = directionVector2.norm();

    // check if there is a zero vector
    if (lenVec1 == 0.f || lenVec2 == 0.f)
    {
      return false;
    }

    float arg = directionVector1.dot(directionVector2) / (lenVec1 * lenVec2);
    if (returnSmallestAngle)
    {
      arg = std::abs(arg);
    }
    // calculate the angle in radians
    angle = arg < -1.f ? M_PI : arg > 1.f ? 0.f : std::acos(arg);
    return true;
  }

  /**
   * @brief getAngleBetween calculates the angle between two infinite straight lines
   * @param line1Point1 first point of first infinite straight line
   * @param line1Point2 second point of first infinite straight line
   * @param line2Point1 first point of second infinite straight line
   * @param line2Point2 second point of second infinite straight line
   * @param[out] angle the calculated angle
   * @param returnSmallestAngle whether return the smallest angle (between 0 and 90 degrees)
   * @return bool whether the calculation was successfull
   */
  inline bool getAngleBetween(const Vector2f& line1Point1, const Vector2f& line1Point2,
                              const Vector2f& line2Point1, const Vector2f& line2Point2,
                              float& angle, bool returnSmallestAngle = true)
  {
    Vector2f vec1 = line1Point1 - line1Point2;
    Vector2f vec2 = line2Point1 - line2Point2;
    return getAngleBetween(vec1, vec2, angle, returnSmallestAngle);
  }

  /**
   * @brief getAngleBetween calculates the angle between two infinite straight lines
   * @param line1 an infinite straight line
   * @param line2 an infinite straight line
   * @param[out] angle the calculated angle
   * @param returnSmallestAngle whether return the smallest angle (between 0 and 90 degrees)
   * @return bool whether the calculation was successfull
   */
  inline bool getAngleBetween(const Line<float>& line1, const Line<float>& line2, float& angle,
                              bool returnSmallestAngle = true)
  {
    Vector2f vec1 = line1.p1 - line1.p2;
    Vector2f vec2 = line2.p1 - line2.p2;
    return getAngleBetween(vec1, vec2, angle, returnSmallestAngle);
  }

  /**
   * @brief getIntersection calculates the intersection between two infinite straight lines
   * @param line1Point1 first point of first infinite straight line
   * @param line1Point2 second point of first infinite straight line
   * @param line2Point1 first point of second infinite straight line
   * @param line2Point2 second point of second infinite straight line
   * @param[out] intersection the calculated point of intersection, not changed if not intersecting
   * @return bool whether they intersect with each other
   */
  template <typename T>
  bool getIntersection(const Vector2<T>& line1Point1, const Vector2<T>& line1Point2,
                       const Vector2<T>& line2Point1, const Vector2<T>& line2Point2,
                       Vector2<T>& intersection)
  {
    float denominator = (line1Point2.y() - line1Point1.y()) * (line2Point2.x() - line2Point1.x()) -
                        (line2Point2.y() - line2Point1.y()) * (line1Point2.x() - line1Point1.x());
    if (!denominator)
    {
      return false;
    }
    else
    {
      intersection.x() =
          ((line1Point2.x() - line1Point1.x()) *
               (line2Point2.x() * line2Point1.y() - line2Point1.x() * line2Point2.y()) -
           (line2Point2.x() - line2Point1.x()) *
               (line1Point2.x() * line1Point1.y() - line1Point1.x() * line1Point2.y())) /
          denominator;

      intersection.y() =
          ((line2Point1.y() - line2Point2.y()) *
               (line1Point2.x() * line1Point1.y() - line1Point1.x() * line1Point2.y()) -
           (line1Point1.y() - line1Point2.y()) *
               (line2Point2.x() * line2Point1.y() - line2Point1.x() * line2Point2.y())) /
          denominator;
    }
    return true;
  }

  /**
   * @brief getIntersection calculates the intersection between two infinite straight lines
   * @param line1 an infinite straight line
   * @param line2 an infinite straight line
   * @param[out] intersection the calculated point of intersection, not changed if not intersecting
   * @return bool whether they intersect with each other
   */
  template <typename T>
  bool getIntersection(const Line<T>& line1, const Line<T>& line2, Vector2<T>& intersection)
  {
    return getIntersection(line1.p1, line1.p2, line2.p1, line2.p2, intersection);
  }

  /**
   * @brief Calculate on which side of a line a point is
   * (https://stackoverflow.com/questions/1560492/how-to-tell-whether-a-point-is-to-the-right-or-left-side-of-a-line)
   * @param linePoint1 point in the line to which the side is to be calculated
   * @param linePoint2 point in the line to which the side is to be calculated
   * @param p the point for which the side shall be calculated
   * @return 1 for left and -1 for right
   */
  inline int sideOfLine(const Vector2f& linePoint1, const Vector2f& linePoint2, const Vector2f& p)
  {
    return (linePoint2.x() - linePoint1.x()) * (p.y() - linePoint1.y()) -
                       (linePoint2.y() - linePoint1.y()) * (p.x() - linePoint1.x()) <
                   0
               ? -1
               : 1;
  }

  /**
   * @brief getSquaredLineDistance returns the shortest distance between a point and an infinite
   * straight line
   *
   * Note that this returns std::numeric_limits<int>::max()/min() in case denominator == 0
   *
   * @param linePoint1 point in the line to which the distance is to be calculated
   * @param linePoint2 point in the line to which the distance is to be calculated
   * @param p the point from which the distance is to be calculated
   */
  inline int getSquaredLineDistance(const Vector2i& linePoint1, const Vector2i& linePoint2,
                                    const Vector2i& p)
  {
    long long int nominator = (linePoint2.y() - linePoint1.y()) * p.x() -
                              (linePoint2.x() - linePoint1.x()) * p.y() +
                              linePoint2.x() * linePoint1.y() - linePoint2.y() * linePoint1.x();
    long long int denominator = (linePoint2 - linePoint1).squaredNorm();
    assert(denominator != 0);
    if (denominator == 0)
    {
      // Return min/max int in case we compiled without assertions
      return nominator >= 0 ? std::numeric_limits<int>::max() : std::numeric_limits<int>::min();
    }
    return (nominator * nominator) / denominator;
  }

  /**
   * @brief getSquaredLineDistance returns the shortest distance between a point and an infinite
   * straight line
   * @param l the line to which the distance is to be calculated
   * @param p the point from which the distance is to be calculated
   */
  inline int getSquaredLineDistance(const Line<int>& l, const Vector2i& p)
  {
    return getSquaredLineDistance(l.p1, l.p2, p);
  }

  /**
   * @brief getSquaredLineDistance returns the shortest distance between a point and an infinite
   * straight line
   *
   * Note that this returns std::numeric_limits<float>::max()/min() in case denominator == 0
   *
   * @param linePoint1 point in the line to which the distance is to be calculated
   * @param linePoint2 point in the line to which the distance is to be calculated
   * @param p the point from which the distance is to be calculated
   */
  inline float getSquaredLineDistance(const Vector2f& linePoint1, const Vector2f& linePoint2,
                                      const Vector2f& p)
  {
    double nominator = (linePoint2.y() - linePoint1.y()) * p.x() -
                       (linePoint2.x() - linePoint1.x()) * p.y() + linePoint2.x() * linePoint1.y() -
                       linePoint2.y() * linePoint1.x();
    double denominator = (linePoint2 - linePoint1).squaredNorm();
    assert(denominator != 0.0);
    if (denominator == 0.0)
    {
      // Return min/max float in case we compiled without assertions
      return nominator >= 0 ? std::numeric_limits<float>::max() : std::numeric_limits<float>::min();
    }
    return (nominator * nominator) / denominator;
  }

  /**
   * @brief getSquaredLineDistance returns the shortest distance between a point and an infinite
   * straight line
   * @param l the line to which the distance is to be calculated
   * @param p the point from which the distance is to be calculated
   */
  inline float getSquaredLineDistance(const Line<float>& l, const Vector2f& p)
  {
    return getSquaredLineDistance(l.p1, l.p2, p);
  }

  /**
   * @brief distPointToLine returns the (NON-SQUARED) distance of a point p to a line l
   * @param linePoint1 point on infinitely long line
   * @param linePoint2 point on infinitely long line
   * @param point the point whichs distance to the aforementioned line is to be determined
   * @return the orthogonal distance of the point to the line
   */
  inline int distPointToLine(const Vector2i& linePoint1, const Vector2i& linePoint2,
                             const Vector2i& point)
  {
    return std::sqrt(getSquaredLineDistance(linePoint1, linePoint2, point));
  }

  /**
   * @brief distPointToLine returns the (NON-SQUARED) distance of a point p to a line l
   * @param line the infinitely long line
   * @param point the point whichs distance to the aforementioned line is to be determined
   * @return the orthogonal distance of the point to the line
   */
  inline int distPointToLine(const Line<int>& line, const Vector2i& point)
  {
    return std::sqrt(getSquaredLineDistance(line, point));
  }

  /**
   * @brief distPointToLine returns the (NON-SQUARED) distance of a point p to a line l
   * @param linePoint1 point on infinitely long line
   * @param linePoint2 point on infinitely long line
   * @param point the point whichs distance to the aforementioned line is to be determined
   * @return the orthogonal distance of the point to the line
   */
  inline float distPointToLine(const Vector2f& linePoint1, const Vector2f& linePoint2,
                               const Vector2f& point)
  {
    return std::sqrt(getSquaredLineDistance(linePoint1, linePoint2, point));
  }

  /**
   * @brief distPointToLine returns the (NON-SQUARED) distance of a point p to a line l
   * @param line the infinitely long line
   * @param point the point whichs distance to the aforementioned line is to be determined
   * @return the orthogonal distance of the point to the line
   */
  inline float distPointToLine(const Line<float>& line, const Vector2f& point)
  {
    return std::sqrt(getSquaredLineDistance(line, point));
  }

  /**
   * @brief http://stackoverflow.com/a/1501725/2169988 (find the shortest distance between a point
   * and a line segment) (modified for squared distance)
   * @param lineSegement the line segment to get the squared distance to
   * @param point a point of which the squared distance to a line is to be computed
   * @return shortest squared distance between point and line segment
   */
  template<typename T>
  inline T getSquaredLineSegmentDistance(const Line<T>& lineSegment, const Vector2<T>& point)
  {
    const T l2 = (lineSegment.p2 - lineSegment.p1).squaredNorm();
    if (l2 == 0.0)
    {
      return (point - lineSegment.p1).squaredNorm();
    }

    // Consider the line extending the segment, parameterized as p1 + t * (p2 - p1).
    // We find projection of point "point" onto the line.
    // It falls where t = [(p - p1) . (p2 - p1)] / |p2 - p1|^2

    const T t = (point - lineSegment.p1).dot(lineSegment.p2 - lineSegment.p1) / l2;

    if (t < 0.0)
    {
      return (point - lineSegment.p1).squaredNorm();
    }
    else if (t > 1.0)
    {
      return (point - lineSegment.p2).squaredNorm();
    }
    const Vector2f projection = lineSegment.p1 + (lineSegment.p2 - lineSegment.p1) * t;

    return (point - projection).squaredNorm();
  }

  /**
   * @brief find the shortest distance between a point and a line segment
   * @param lineSegement the line segment to get the distance to
   * @param point a point which distance to a line is to be computed
   * @return shortest distance between point and line segment
   */
  template<typename T>
  inline T getLineSegmentDistance(const Line<T>& lineSegment, const Vector2<T>& point)
  {
    return std::sqrt(getSquaredLineSegmentDistance(lineSegment, point));
  }

  /**
   * @brief distLineSegmentToLineSegment calculates the shortest distance between two line segments
   * http://geomalgorithms.com/a07-_distance.html#dist3D_Segment_to_Segment
   * @param line1Point1 first end point of first line segment
   * @param line1Point2 second end point of first line segment
   * @param line2Point1 first end point of second line segment
   * @param line2Point2 second end point of second line segment
   * @return the shortest distance between two line segments
   */
  inline float distLineSegmentToLineSegment(const Vector2f& line1Point1,
                                            const Vector2f& line1Point2,
                                            const Vector2f& line2Point1,
                                            const Vector2f& line2Point2)
  {
    float smallNum = 0.00000001;
    Vector2f u = line1Point2 - line1Point1;
    Vector2f v = line2Point2 - line2Point1;
    Vector2f w = line1Point1 - line2Point1;
    float a = u.dot(u); // always >= 0
    float b = u.dot(v);
    float c = v.dot(v); // always >= 0
    float d = u.dot(w);
    float e = v.dot(w);
    float D = a * c - b * b; // always >= 0
    float sc, sN, sD = D;    // sc = sN / sD, default sD = D >= 0
    float tc, tN, tD = D;    // tc = tN / tD, default tD = D >= 0

    // compute the line parameters of the two closest points
    if (D < smallNum)
    {           // the lines are almost parallel
      sN = 0.0; // force using point P0 on segment S1
      sD = 1.0; // to prevent possible division by 0.0 later
      tN = e;
      tD = c;
    }
    else
    { // get the closest points on the infinite lines
      sN = (b * e - c * d);
      tN = (a * e - b * d);
      if (sN < 0.0)
      { // sc < 0 => the s=0 edge is visible
        sN = 0.0;
        tN = e;
        tD = c;
      }
      else if (sN > sD)
      { // sc > 1  => the s=1 edge is visible
        sN = sD;
        tN = e + b;
        tD = c;
      }
    }

    if (tN < 0.0)
    { // tc < 0 => the t=0 edge is visible
      tN = 0.0;
      // recompute sc for this edge
      if (-d < 0.0)
        sN = 0.0;
      else if (-d > a)
        sN = sD;
      else
      {
        sN = -d;
        sD = a;
      }
    }
    else if (tN > tD)
    { // tc > 1  => the t=1 edge is visible
      tN = tD;
      // recompute sc for this edge
      if ((-d + b) < 0.0)
        sN = 0;
      else if ((-d + b) > a)
        sN = sD;
      else
      {
        sN = (-d + b);
        sD = a;
      }
    }
    // finally do the division to get sc and tc
    sc = (std::abs(sN) < smallNum ? 0.0 : sN / sD);
    tc = (std::abs(tN) < smallNum ? 0.0 : tN / tD);

    // get the difference of the two closest points
    Vector2f dP = w + (sc * u) - (tc * v); // =  S1(sc) - S2(tc)

    return dP.norm(); // return the closest distance
  }

  /**
   * @brief distLineSegmentToLineSegment calculates the shortest distance between two line segments
   * @param lineSegment1 the first line segment
   * @param lineSegment2 the second line segment
   * @return the shortest distance between two line segments
   */
  inline float distLineSegmentToLineSegment(const Line<float>& lineSegment1,
                                            const Line<float>& lineSegment2)
  {
    return distLineSegmentToLineSegment(lineSegment1.p1, lineSegment1.p2, lineSegment2.p1,
                                        lineSegment2.p2);
  }

  /**
   * @brief check if an object is inside an ellipse
   * @param objectPosition position vector of said object
   * @param ellipseCenter the center of said ellipse
   * @param semiAxisX the ellipse semi axis in x direction
   * @param semiAxisY the ellipse semi axis in y direction
   * @param objectInEllipseThreshold the threshold for being inside the ellipse
   * @return whether the object is inside the ellipse or not
   */
  inline bool isInsideEllipse(const Vector2f& objectPosition, const Vector2f& ellipseCenter,
                              const float semiAxisX, const float semiAxisY,
                              const float objectInEllipseThreshold)
  {
    const Vector2f centerToObject = objectPosition - ellipseCenter;
    return (Vector2f(centerToObject.x() / semiAxisX, centerToObject.y() / semiAxisY).squaredNorm() <
            objectInEllipseThreshold * objectInEllipseThreshold);
  }

  /**
   * @enum CircleIntersectionType enumerates the return types of getCircleIntersection
   * @brief NO_INTERSECTION: given circles have no intersections
   *        ONE_INTERSECTION: given circles have exact one intersection (tangent)
   *        TWO_INTERSECTIONS: given circles have two intersections
   *        INF_INTERSECTIONS: given circles are the same
   */
  enum class CircleIntersectionType
  {
    NO_INTERSECTION,
    ONE_INTERSECTION,
    TWO_INTERSECTIONS,
    INF_INTERSECTIONS
  };

  template <typename T>
  inline CircleIntersectionType
  getCircleIntersection(const Circle<T>& circle1, const Circle<T>& circle2,
                        std::pair<Vector2<T>, Vector2<T>>& intersection)
  {
    const T squaredDistance((circle2.center - circle1.center).squaredNorm());
    const T radiusSum = circle1.radius + circle2.radius;
    if (squaredDistance > radiusSum * radiusSum)
    {
      // no solutions, circles are seperate
      return CircleIntersectionType::NO_INTERSECTION;
    }
    const T radiusDifference = circle1.radius - circle2.radius;
    if (squaredDistance < radiusDifference * radiusDifference)
    {
      // no solutions, circle is contained within the other
      return CircleIntersectionType::NO_INTERSECTION;
    }
    if (squaredDistance == 0 && circle1.radius == circle2.radius)
    {
      // no solutions circles are coincident -> inifite number of solutions
      return CircleIntersectionType::INF_INTERSECTIONS;
    }
    const T distance = std::sqrt(squaredDistance);
    const T center1ToIntersectionLine(
        (circle1.radius * circle1.radius - circle2.radius * circle2.radius + squaredDistance) /
        (2 * distance));
    const T intersectionLineHeight(std::sqrt(circle1.radius * circle1.radius -
                                       center1ToIntersectionLine * center1ToIntersectionLine));
    Vector2<T> middleOnIntersectionLine(circle1.center + center1ToIntersectionLine / distance *
                                                             (circle2.center - circle1.center));

    if (distance == circle1.radius + circle2.radius)
    {
      intersection.first = middleOnIntersectionLine;
      return CircleIntersectionType::ONE_INTERSECTION;
    }

    Vector2<T> orthogonalToIntersectionLine(circle2.center.y() - circle1.center.y(),
                                            -(circle2.center.x() - circle1.center.x()));

    intersection.first =
        middleOnIntersectionLine + intersectionLineHeight / distance * orthogonalToIntersectionLine;
    intersection.second =
        middleOnIntersectionLine - intersectionLineHeight / distance * orthogonalToIntersectionLine;
    return CircleIntersectionType::TWO_INTERSECTIONS;
  }

  /**
   * @brief getTangentPointsOfCircle calculates the points where lines starting at a point are
   * tangent to a circle with given center and radius
   * @param startPoint the point where the tangent line should be start
   * @param circleCenter the center of the circle where the lines should be tangent to
   * @param circleRadius the radius of the circle where the lines should be tangent to
   * @param[out] a pair of the points where the lines are tangent to the circle
   * @return bool whether the calculation worked or not
   */
  template <typename T>
  inline bool getTangentPointsOfCircle(const Vector2<T> startPoint, const Vector2<T> circleCenter,
                                       const float circleRadius,
                                       std::pair<Vector2<T>, Vector2<T>>& tangentPoints)
  {
    const T squaredDistance = (circleCenter - startPoint).squaredNorm();
    if (squaredDistance <= circleRadius * circleRadius)
    {
      // no tangent points possible
      return false;
    }
    // Construct two Circles to get the tangent points
    getCircleIntersection(Circle<T>(startPoint, std::sqrt(squaredDistance)),
                          Circle<T>(circleCenter, circleRadius), tangentPoints);
    // calculated the tangent points
    return true;
  }

  /**
   * @brief clip clips convex polygon with plane
   * @param A convex polygon to clip
   * @param P plane to clip with
   * @return resulting convex polygon
   */
  template <typename T>
  inline ConvexPolygon<T> clip(const ConvexPolygon<T>& A, const Plane<T>& P)
  {
    ConvexPolygon<T> R; // resulting polygon

    for (unsigned int i = 0, j = A.points.size() - 1; i < A.points.size(); j = i, i++)
    {
      Vector2<T> V = A.points[j];
      Vector2<T> W = A.points[i];

      float sign_v = (V - P.origin).dot(P.normal);
      float sign_w = (W - P.origin).dot(P.normal);

      // both points outside the half-plane
      if (sign_v > 0.0f && sign_w > 0.0f)
      {
        continue;
      }

      // edge start inside the half plane. add it.
      if (sign_v <= 0.0f)
      {
        R.points.push_back(V);
      }

      // edge intersected by plane.
      if ((sign_v < 0.0f && sign_w > 0.0f) || (sign_v > 0.0f && sign_w < 0.0f))
      {
        float t = -sign_v / (sign_w - sign_v); // intersection param

        Vector2f Q = V.template cast<float>() +
                     t * (W - V).template cast<float>(); // intersection of edge with plane

        R.points.push_back(Q.cast<T>());
        continue;
      }
    }
    return R;
  }

  /**
   * @brief intersect calculates intersection of two convex polygons.
   * Make sure that the points ordered counterclockwise.
   * https://www.gamedev.net/forums/topic/518779-oriented-bounding-box---finding-overlap-area/?tab=comments#comment-4370201
   * @param A first convex polygon
   * @param B second convex polygon
   * @param[out] intersection intersection polygon (convex)
   * return whether they intersect
   */
  template <typename T>
  inline bool intersect(const ConvexPolygon<T>& A, const ConvexPolygon<T>& B,
                        ConvexPolygon<T>& intersection)
  {
    intersection = B; // copy polygon B to result intersection

    for (unsigned int i = 0, j = A.points.size() - 1; i < A.points.size(); j = i, i++)
    {
      Vector2<T> E(A.points[i] - A.points[j]); // edge of polygon A
      // plane normal (use opposite if your vertex winding is clockwise).
      Vector2<T> N(-E.y(), E.x());
      Vector2<T> O(A.points[i]); // plane origin
      Plane<T> P(O, N);          // infinite plane passing through edge

      intersection = clip(intersection, P); // 'cut' polygon intersection with plane P
    }

    if (intersection.points.size() != 0)
    {
      return true;
    }
    else
    {
      return false;
    }
  }

  /**
   * @brief intersectionOverUnion calculcates the intersection over union over two polygons
   * @param A first convex polygon
   * @param B second convex polygon
   * @return the intersection over union
   */
  template <typename T>
  inline float intersectionOverUnion(const ConvexPolygon<T>& A, const ConvexPolygon<T>& B)
  {
    ConvexPolygon<T> intersection;
    intersect(A, B, intersection);
    float intersectionArea = intersection.area();
    return intersectionArea / (A.area() + B.area() - intersectionArea);
  }

  /**
   * @brief Calculates the percentage of the overlapping area to the total area of the second
   * polygon.
   * @param firstPolygon first convex polygon
   * @param secondPolygon second convex polygon
   * @return percentage share
   */
  template <typename T>
  float percentageOfIntersection(const ConvexPolygon<T>& firstPolygon,
                                 const ConvexPolygon<T>& secondPolygon)
  {
    ConvexPolygon<T> intersection;
    Geometry::intersect(firstPolygon, secondPolygon, intersection);
    return intersection.area() / secondPolygon.area();
  }

} // namespace Geometry
