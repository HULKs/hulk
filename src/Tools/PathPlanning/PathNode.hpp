#pragma once

#include "Data/TeamObstacleData.hpp"
#include "Libs/AStarSearch/AStarSearch.hpp"
#include "Tools/Math/Geometry.hpp"
#include <atomic>
#include <mutex>


/**
 *  @brief Node Type describes what kind of object the pair of tangent a node belongs to
 */
enum class NodeType
{
  OBSTACLE = 0,
  START = 1,
  GOAL = 2
};

/**
 * @brief operator>> allow streaming to this type
 */
inline void operator>>(const Uni::Value& in, NodeType& out)
{
  out = static_cast<NodeType>(in.asInt32());
}

/**
 * @brief operator<< allow streaming from this type
 */
inline void operator<<(Uni::Value& out, const NodeType in)
{
  out << static_cast<int>(in);
}

class PathNode;

struct PathObstacle : public Uni::To
{
  // A list of successors which are located on this obstacle
  std::vector<std::shared_ptr<PathNode>> nodesOnObstacle;
  // A list of arcs blocked by objects
  std::vector<Arc<float>> blockedArcs;
  // Whether this obstacle is populated
  bool isPopulated{false};
  // A Circle representing the obstacle
  Circle<float> circle;

  explicit PathObstacle(const Vector2f& position, const float radius)
    : circle(position, radius)
  {
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["circlePosition"] << circle.center;
    value["radius"] << circle.radius;
    value["blockedArcs"] << blockedArcs;
  }

  bool operator==(const PathObstacle& other) const
  {
    return (circle.center == other.circle.center && circle.radius == other.circle.radius);
  }
};


class PathNode : public std::enable_shared_from_this<PathNode>, public Uni::To
{
public:
  /// Id of this node
  unsigned int id{0};
  /// The node's absolute position
  Vector2f absolutePosition;
  /// The obstacle this node is located on
  PathObstacle* connectedObstacle{nullptr};
  /// A pointer to the node on the other end of the tangent
  std::weak_ptr<PathNode> pairNode;
  /// A collection of stored arcs to other nodes on the same connectedObstacle
  std::unordered_map<unsigned int, Arc<float>> storedArcs;
  /// direction the connected obstacle would be circumnavigated in when entering through this node
  bool clockwise{false};
  /// pointer to all path obstacles
  std::vector<PathObstacle>* pathObstacles{nullptr};
  /// the type of this node's pair node i.e. the node this node was generated from
  NodeType nodeType{NodeType::OBSTACLE};
  /// whether this node is the node path planning is aiming for
  bool isGoalNode{false};


  PathNode(unsigned int id, Vector2f absolutePosition, std::vector<PathObstacle>& pathObstacles,
           PathObstacle* connectedObstacle, std::weak_ptr<PathNode> pairNode, bool isGoal);

  // interfacing to the A* library
  /**
   * @brief determines wether a given other PathNode is the same as this node
   * @param other an other PathNode to compare to
   * @return wether other and this node is the same node
   */
  bool isSameNode(const std::shared_ptr<PathNode>& other) const;
  /**
   * @brief isGoal determines wether a given goal node is this node
   * @param the goal node to compare to
   * @return wether this node is the given goal node
   */
  bool isGoal(const std::shared_ptr<PathNode>& nodeGoal) const;
  /**
   * @brief heuristic function that estimates the distance from a this PathNode to the goal
   * @param nodeGoal the goal node this function should estimate the distance to
   * @return the estimated distance from this node to the given goal node
   */
  float goalDistanceEstimate(const std::shared_ptr<PathNode>& nodeGoal);
  /**
   * @brief This generates the successors
   * @param aStarSeach a pointer to the A* object searching for the path
   * @param parentNode the node this node is a successor of
   * @param goalNode the PathNode PathPlanning is aiming for
   * @return bool always true to signal the A* search to continue
   */
  bool getSuccessors(AStarSearch<PathNode>* aStarSearch,
                     const std::shared_ptr<PathNode>& parentNode,
                     const std::shared_ptr<PathNode>& goalNode);
  /**
   * @brief calculates the cost from this node to a given successor
   * @param successor the successor to plan to
   * @return the cost (walk distance in [m])
   */
  float getCost(const std::shared_ptr<PathNode>& successor);

  // helping methods
  void expandNodeToObstacle(
      AStarSearch<PathNode>* aStarSearch, PathObstacle* pathObstacle,
      const std::shared_ptr<PathNode>&
          node); /**
                  * @brief Wraps expandNodeToObstacle to perform checks whether the tangents are
                  * already calculated
                  * @param aStarSearch a pointer to the A* object searching for the path
                  */
  void expandNotOnObstacle(AStarSearch<PathNode>* aStarSearch);
  /**
   * @brief Explore and add nodes from this node
   * @param aStarSearch a pointer to the A* object searching for the path
   * @param parentNode the previous in the currently explored path
   * @param goalNode the node path planning is aiming for
   */
  void expandOnObstacle(AStarSearch<PathNode>* aStarSearch,
                        const std::shared_ptr<PathNode>& parentNode,
                        const std::shared_ptr<PathNode>& goalNode);
  /**
   * @brief Check if otherNode is reachable and if so, store the connecting arc. otherNode and this
   * node are assumed to lie on the same obstacle
   * @param otherNode the other node to connect to
   * @return whether a free arc was found
   */
  bool findAndStoreFreeArc(const std::shared_ptr<PathNode>& otherNode);
  /**
   * @brief Checks whether a given point is reachable from this node (i.e. line of sight is not
   * obstructed by any obstacle)
   * @param start the position to start the search line from
   * @param target the position to reach for
   * @bool whether the point is reachable
   */
  bool isReachable(const Vector2f& start, const Vector2f& target) const;

  /// @brief prints debugging information about this node to stdout
  void printNodeInfo();

  void toValue(Uni::Value& value) const override;
};

/// @brief PathNodeFactory constructs PathNodes with incrementing ids
class PathNodeFactory
{
public:
  /**
   * @brief create a new PathNode with given parameters
   * @see PathNode() constructor
   * @return a shared poiter to the newly created PathNode
   */
  static std::shared_ptr<PathNode>
  create(Vector2f absolutePosition, std::vector<PathObstacle>& pathObstacles,
         PathObstacle* connectedObstacle,
         std::weak_ptr<PathNode> pairNode = std::weak_ptr<PathNode>{}, bool isGoal = false);
  /// @brief resets the id counter to 0
  static void resetCounter();

private:
  /// state to name new PathNodes uniquely with an always incrementing id
  static thread_local std::atomic_uint32_t idCounter__;
};
