import AppKit

// Put a rich HTML fragment on the pasteboard so a paste into Google Docs
// carries headings, tables and inline (data-URI) images in one operation.
let path = CommandLine.arguments[1]
let html = try! String(contentsOfFile: path, encoding: .utf8)
let pb = NSPasteboard.general
pb.clearContents()
pb.declareTypes([.html, .string], owner: nil)
pb.setString(html, forType: .html)
pb.setString("The Sauce Book", forType: .string)
print("clipboard: \(html.count) chars of text/html")
