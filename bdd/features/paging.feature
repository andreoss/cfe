Feature: More than fits on a page
  When a subject has more remarks than one page holds, the rest are
  still reachable, and a later page holds remarks the first did not.

  Scenario: A long subject is offered a page at a time
    Given the board is answering
    And there is a subject to read
    When an administrator signs in
    And they add enough remarks to fill more than one page
    Then the first page does not hold every one of them
    And a later page holds remarks the first page did not
