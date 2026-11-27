#include "shim.h"

#include <giac/config.h>
#include <giac/giac.h>

#include <stdexcept>
#include <string>

namespace pennoa {

struct SessionImpl {
  giac::context ctx;
};

Session::Session() : impl(std::make_unique<SessionImpl>()) {}
Session::~Session() = default;

std::unique_ptr<Session> new_session() {
  return std::make_unique<Session>();
}

rust::String eval(Session &session, rust::Str expr) {
  std::string input(expr.data(), expr.size());
  giac::context *ctx = &session.impl->ctx;
  giac::gen parsed(input, ctx);
  // Giac reports parse failures as a gen of type _STRNG or with an error flag;
  // genuine evaluation errors are thrown as std::runtime_error (mapped by cxx).
  if (giac::first_error_line(ctx)) {
    throw std::runtime_error("parse error near: " + giac::error_token_name(ctx));
  }
  giac::gen result = giac::protecteval(parsed, 1, ctx);
  std::string out = result.print(ctx);
  if (out.rfind("Error:", 0) == 0) {
    throw std::runtime_error(out);
  }
  return rust::String(out);
}

} // namespace pennoa
