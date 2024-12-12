# Natural Language Syntax Exploration

# Type System Refinement
count as Whole
measure as Decimal is 3.14
message as Text 
flag as Logic
empty as Void
empty is null

count is 0
message as Text is "Hello, World!"

# Data Structures
number_collection is [1, 2, 3, 4, 5] as List[Whole]
person_info as Mapping \
    of Text to Any is \
    "name": Text is "John",
    "age": Whole is 30,
    "is_student": Logic is false

# Compute Block with Descriptive Parameters
Job process requires first, second, action as Number, Number, Text returning Whole:
    when action is "add":
        output first + second
    when action is "multiply":
        output first * second
    or:
        raise "Unknown action" as Error

# Entity Definition with Inheritance
Object Person inherits BaseEntity:
    build defaults name as Text is "Unknown", age as Whole is 0:
        my name is name
        my age is age
    
    Job greetings returns Text:
        output "I am {my name}, {my age} years old"
    
    Job anonymous returns Person:
        output new Person()

# Handling Potential Errors
Job possible_risk:
    do:
        result is 10 / 0
    fail problem as DivideByZeroError:
        show "Encountered: {problem.message}"
    always:
        show "Examination finished"

# Functional Transformations
numbers is [1, 2, 3, 4, 5] as List[Whole]
squared is numbers and each number becomes number * number // for each collect
filtered is numbers when each number > 2 // iter map filter

# Async Data Retrieval
Job gather_data requires url as Text returns Promise[Text]:
    response as Text is await http.fetch at url
    output response.content

# Understanding Different Values
Job describe_value requires value as Any returns Text:
    match value:
        when Whole:
            output "This is a whole number"
        when Decimal: 
            output "This is a measured number"
        when Text: 
            output "This is a message"
        or: 
            output "Unknown type"

# Sequence Generator # TODO: Implement an iterator and reword this to use it
Job fibonacci requires max as Whole returns Whole:
    first as Whole is 0
    second as Whole is 1
    
    loop while true:
        yield first
        next as Whole is first + second
        first is second
        second is next
        show "first: {first}, second: {second}"
        
        when first > max:
            output first

# TODO: Create a Lambda type

# Primary Execution
main:
    person as Person is new Person with "Alice", 25
    show person speak about me
    
    result as Whole is process using 5, 3, "multiply"
    show "Calculation outcome: {result}"
    
    show "Fibonacci sequence up to 100:" as Text
    fibonacci using 100

    show "Describe value:"
    show describe_value using 10
    show describe_value using 3.14
    show describe_value using "Hello, World!"
    show describe_value using true

    show "Gather data:"
    gather_data using "https://example.com"