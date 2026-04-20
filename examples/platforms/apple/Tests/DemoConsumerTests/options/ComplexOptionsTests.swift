import Demo
import XCTest

final class ComplexOptionsTests: XCTestCase {
    func testComplexOptionFns() {
        XCTAssertEqual(echoOptionalString(v: "hello"), "hello")
        XCTAssertNil(echoOptionalString(v: nil))
        XCTAssertEqual(isSomeString(v: "x"), true)
        XCTAssertEqual(isSomeString(v: nil), false)

        XCTAssertEqual(echoOptionalPoint(v: Point(x: 1.0, y: 2.0)), Point(x: 1.0, y: 2.0))
        XCTAssertNil(echoOptionalPoint(v: nil))
        XCTAssertEqual(makeSomePoint(x: 3.0, y: 4.0), Point(x: 3.0, y: 4.0))
        XCTAssertNil(makeNonePoint())

        XCTAssertEqual(echoOptionalStatus(v: .active), .active)
        XCTAssertNil(echoOptionalStatus(v: nil))
        XCTAssertEqual(echoOptionalVec(v: [1, 2, 3]), [1, 2, 3])
        XCTAssertNil(echoOptionalVec(v: nil))
        XCTAssertEqual(optionalVecLength(v: [9, 8]), 2)
        XCTAssertNil(optionalVecLength(v: nil))
        XCTAssertEqual(findName(id: 1), "Name_1")
        XCTAssertNil(findName(id: 0))
        XCTAssertEqual(findNumbers(count: 3), [0, 1, 2])
        XCTAssertNil(findNumbers(count: 0))
        XCTAssertEqual(findNames(count: 2), ["Name_0", "Name_1"])
        XCTAssertNil(findNames(count: 0))
        XCTAssertEqual(findApiResult(code: 0), .success)
        XCTAssertEqual(findApiResult(code: 1), .errorCode(-1))
        XCTAssertEqual(findApiResult(code: 2), .errorWithData(code: -1, detail: -2))
        XCTAssertNil(findApiResult(code: 99))
    }
}
