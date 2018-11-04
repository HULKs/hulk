#pragma once

#include "Tools/Math/Eigen.hpp"

#include <boost/geometry.hpp>
#include <boost/geometry/index/rtree.hpp>

#include <boost/geometry.hpp>
#include <boost/geometry/geometries/box.hpp>
#include <boost/geometry/geometries/point.hpp>
#include <boost/geometry/index/rtree.hpp>

namespace bg = boost::geometry;
namespace bgi = boost::geometry::index;
typedef bg::model::point<float, 2, bg::cs::cartesian> BPoint;
typedef bg::model::segment<BPoint> BSegment;
typedef bg::model::box<BPoint> BBox;

/**
 * Implementation of the DBScan Algorithm.
 * (see https://en.wikipedia.org/wiki/DBSCAN#Original_Query-based_Algorithm)
 * @tparam Particle type of the clustered elements. Should be one of {@link BPoint}, {@link
 * BSegment} or {@link BBox}.
 * @tparam ParticleData type of the data corresponding to the clustered element.
 * @author Georg Felbinger
 */
template <typename Particle, typename ParticleData>
class DBScan
{

public:
  /**
   * @brief adds a particle to the rtree.
   * @param the Particle used for the clustering.
   * @param the ParticleData stored alongside with the particle
   */
  void addParticle(const Particle& particle, const ParticleData& data)
  {
    RTreeElement seed = std::make_pair(particle, idCounter++);
    rtree_.insert(seed);
    seeds_.push_back(seed);
    clusterData_.push_back({data, Label::UNDEFINED});
  }

  /**
   * @brief executes the DBScan algorithm.
   * @param minPts algorithm parameter.
   * @param eps algorithm parameter.
   * @param dist algorithm parameter, should return whether two Particles are close together.
   * @param[out] result a vector of clusters.
   */
  void calculateClusters(const unsigned int minPts, const Vector2f& eps,
                         const std::function<bool(const Particle&, const Particle&)>& dist,
                         std::vector<std::vector<std::pair<Particle, ParticleData>>>& result)
  {
    // according to https://en.wikipedia.org/wiki/DBSCAN#Original_Query-based_Algorithm
    for (auto seed : seeds_)
    {
      const Particle& p = seed.first;
      ClusterParticle& s = clusterData_[seed.second];
      // Previously processed in inner loop
      if (s.label != Label::UNDEFINED)
      {
        continue;
      }
      //  Find neighbors
      std::vector<RTreeValue> neighbors;
      const std::function<bool(const RTreeElement&)> curriedDist =
          [&p, &dist](const RTreeElement& q) { return dist(p, q.first); };
      rtree_.query(bgi::intersects(mkBox(p, eps)) && bgi::satisfies(curriedDist),
                   std::back_inserter(neighbors));
      // Density check
      if (neighbors.size() < minPts)
      {
        // Label as noise
        s.label = Label::NOISE;
        continue;
      }
      // Next cluster
      result.emplace_back();
      // Label initial BPoint
      result.back().push_back(std::make_pair(p, s.particleData));
      s.label = Label::CLUSTER;
      // Neighbors to expand, process every seed point
      for (unsigned neighborId = 0; neighborId < neighbors.size(); neighborId++)
      {
        const auto& neighbor = neighbors[neighborId];
        const Particle& np = neighbor.first;
        auto& ns = clusterData_[neighbor.second];
        // Change noise to border point
        if (ns.label == Label::NOISE)
        {
          ns.label = Label::CLUSTER;
          result.back().push_back(std::make_pair(np, ns.particleData));
        }
        // Previously processed
        if (ns.label != Label::UNDEFINED)
        {
          continue;
        }
        // Label neighbor
        ns.label = Label::CLUSTER;
        result.back().push_back(std::make_pair(np, ns.particleData));
        // Find neighbors
        std::vector<RTreeValue> nextNeighbors;
        const std::function<bool(const RTreeElement&)> curriedNeighborDist =
            [&np, &dist](const RTreeElement& q) { return dist(np, q.first); };
        rtree_.query(bgi::intersects(mkBox(np, eps)) && bgi::satisfies(curriedNeighborDist),
                     std::back_inserter(nextNeighbors));
        // Density check
        if (nextNeighbors.size() < minPts)
        {
          continue;
        }
        // Add new neighbors to seed set
        for (auto& nn : nextNeighbors)
        {
          //          if (clusterData_[nn.second].label != Label::UNDEFINED)
          {
            neighbors.push_back(nn);
          }
        }
      }
    }
  }

private:
  typedef std::pair<Particle, unsigned int> RTreeValue;
  enum Label
  {
    UNDEFINED = 0,
    CLUSTER = 1,
    NOISE = 2
  };
  struct ClusterParticle
  {
    const ParticleData particleData;
    Label label;
  };

  unsigned int idCounter = 0;
  typedef std::pair<Particle, unsigned int> RTreeElement;
  bgi::rtree<RTreeElement, bgi::quadratic<16>> rtree_; // TODO linear?
  std::vector<ClusterParticle> clusterData_;
  std::vector<RTreeElement> seeds_;

  BBox mkBox(const BPoint& p, const Vector2f& eps) const
  {
    return BBox(BPoint(p.get<0>() - eps.x(), p.get<1>() - eps.y()),
                BPoint(p.get<0>() + eps.x(), p.get<1>() + eps.y()));
  }
  BBox mkBox(const BSegment& segment, const Vector2f& eps) const
  {
    auto& p1 = segment.first;
    auto& p2 = segment.second;
    return BBox(BPoint(p1.get<0>() - eps.x(), p1.get<1>() - eps.y()),
                BPoint(p2.get<0>() + eps.x(), p2.get<1>() + eps.y()));
  }
  BBox mkBox(const BBox& box, const Vector2f& eps) const
  {
    const BPoint p1(box.min_corner().get<0>() - eps.x(), box.min_corner().get<1>() - eps.y());
    const BPoint p2(box.max_corner().get<0>() - eps.x(), box.max_corner().get<1>() - eps.y());
    return BBox(p1, p2);
  }
};
