Feature: Writing to a board
  Somebody signed in may add to a subject, and what they add is there
  afterwards under their name. Somebody who has not signed in may not.

  Scenario: A remark is added to a subject
    Given the board is answering
    And there is a subject to read
    When an administrator signs in
    And they add a remark to the subject
    Then the remark is on the subject
    And the remark is under their name

  Scenario: Somebody who has not signed in may not add a remark
    Given the board is answering
    And there is a subject to read
    When somebody who has not signed in tries to add a remark
    Then the remark is refused
