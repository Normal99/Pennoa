#pragma once
#include "rust/cxx.h"
#include <memory>

namespace pennoa {

struct SessionImpl; // holds one giac::context; defined in shim.cpp

// One Giac session (not thread-safe; one per thread).
class Session {
public:
  Session();
  ~Session();
  Session(const Session &) = delete;
  Session &operator=(const Session &) = delete;
  std::unique_ptr<SessionImpl> impl;
};

std::unique_ptr<Session> new_session();
rust::String eval(Session &session, rust::Str expr);

} // namespace pennoa
