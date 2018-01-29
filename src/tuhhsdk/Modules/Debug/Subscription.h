#pragma once

#include <map>
#include <string>
#include <vector>

#include <boost/asio.hpp>
#include <mutex>


template <class Subscriber, class KeyType>
class SubscriptionManager
{
public:
  typedef std::vector<KeyType> Subscriptions;
  typedef std::vector<Subscriber> Subscribers;
  typedef std::map<Subscriber, Subscriptions> SubscriberMap;

private:
  SubscriberMap subMap_;

public:
  SubscriptionManager()
    : subMap_()
  {
  }
  ~SubscriptionManager() {}

  /** Subscribes a Subscriber to a Debug-Key
   * If the subscriber exists, the subscription will be added to his subscriptions.
   * If it is already subscribed to this key, nothing will be done.
   * If it does not exist, it will be created and the key will be subscribed to.
   * @param subscriber The subscriber
   * @param key The Key.
   */
  void subscribe(const Subscriber& subscriber, const KeyType& key)
  {
    // check, if subscriber exists already...
    auto subIt = subMap_.find(subscriber);
    if (subIt != subMap_.end())
    {
      // check, if key is not subscribed to, yet...
      Subscriptions& keys = subIt->second;
      auto keyIt = std::find(keys.begin(), keys.end(), key);
      if (keyIt == keys.end())
      {
        keys.push_back(key);
      }
    }
    else
    {
      Subscriptions keys = {key};
      subMap_[subscriber] = keys;
    }
  }

  void subscribe(const Subscriber& subscriber, const Subscriptions& subscriptions)
  {
    // check, if subscriber exists already...
    auto subIt = subMap_.find(subscriber);
    if (subIt != subMap_.end())
    {
      if (subscriptions.empty())
      {
        subMap_.erase(subIt);
        return;
      }

      subMap_[subscriber] = subscriptions;
    }
    else if (!subscriptions.empty())
    {
      Subscriptions s = subscriptions;
      subMap_[subscriber] = s;
    }
  }

  /** Unsubscribes a Subscriber from a Debug-Key
   * It is made sure, that the subscriber is not subscribed to the key.
   * @param subscriber The Subscriber
   * @param key The Key.
   */
  void unsubscribe(const Subscriber& subscriber, const KeyType& key)
  {
    auto subIt = subMap_.find(subscriber);
    if (subIt != subMap_.end())
    {
      // check, if key is existing, then delete it.
      Subscriptions& keys = subIt->second;
      auto keyIt = std::find(keys.begin(), keys.end(), key);
      if (keyIt != keys.end())
      {
        keys.erase(keyIt);
      }

      // Delete subscriber, if it has no keys.
      if (keys.empty())
      {
        subMap_.erase(subIt);
      }
    }
  }

  /** The Subscriber will be deleted
   * Thus the subscriber is gone, so it will not be subscribed to anything.
   * @param subscriber The Subscriber.
   */
  void deleteSubscriber(const Subscriber& subscriber)
  {
    auto subIt = subMap_.find(subscriber);
    subMap_.erase(subIt);
  }

  /** Returns a Vector with all Subscriptions of a Subscriber
   * The vector is a std::vector<KeyType>
   * @param subscriber The Subscriber, you want to know the subscriptions of.
   */
  Subscriptions& getSubscriptions(Subscriber subscriber)
  {
    return subMap_[subscriber];
  }

  Subscribers getSubscribers()
  {
    Subscribers result;
    for (auto it = subMap_.begin(); it != subMap_.end(); ++it)
    {
      result.push_back(it->first);
    }

    return result;
  }
};
