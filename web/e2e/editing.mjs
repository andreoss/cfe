import { Builder, By, until } from 'selenium-webdriver'
import chrome from 'selenium-webdriver/chrome.js'

const baseUrl = process.env.BASE_URL ?? 'http://127.0.0.1:58081'
const chromedriverPath = process.env.CHROMEDRIVER_PATH

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

async function buildDriver() {
  const options = new chrome.Options()
  options.addArguments('--headless=new', '--no-sandbox', '--disable-gpu')
  const builder = new Builder().forBrowser('chrome').setChromeOptions(options)
  if (chromedriverPath) {
    const service = new chrome.ServiceBuilder(chromedriverPath)
    builder.setChromeService(service)
  }
  return builder.build()
}

async function registerAndPost(driver, username, title, body) {
  await driver.get(`${baseUrl}/register`)
  await driver.wait(until.elementLocated(By.name('username')), 5000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('email')).sendKeys(`${username}@example.com`)
  await driver.findElement(By.name('password')).sendKeys('correcthorse')
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(until.urlIs(`${baseUrl}/`), 5000)

  await driver.findElement(By.linkText('General')).click()
  await driver.wait(until.urlIs(`${baseUrl}/s/general`), 5000)
  await driver.findElement(By.xpath("//button[text()='New topic']")).click()
  await driver.findElement(By.name('title')).sendKeys(title)
  await driver.findElement(By.name('body')).sendKeys(body)
  await driver.findElement(By.xpath("//button[text()='Post']")).click()
  await driver.wait(until.elementLocated(By.linkText(title)), 5000)
  await driver.findElement(By.linkText(title)).click()
  await driver.wait(until.elementLocated(By.name('comment-body')), 5000)
  return driver.getCurrentUrl()
}

async function run() {
  const author = await buildDriver()
  const suffix = Date.now().toString(36)
  const authorName = `e2e_edit_a_${suffix}`
  const strangerName = `e2e_edit_b_${suffix}`
  const originalTitle = `Revisable subject ${suffix}`
  const editedTitle = `Revised subject ${suffix}`
  const originalComment = `Original comment ${suffix}`
  const editedComment = `Edited comment ${suffix}`
  let stranger
  try {
    const topicUrl = await registerAndPost(
      author,
      authorName,
      originalTitle,
      'Body before the edit.',
    )

    let bodyText = await author.findElement(By.css('body')).getText()
    assert(!bodyText.includes('(edited)'), 'a fresh topic must not be marked edited')

    await author.findElement(By.xpath("//button[text()='Edit topic']")).click()
    const titleBox = await author.wait(until.elementLocated(By.name('edit-title')), 5000)
    await titleBox.clear()
    await titleBox.sendKeys(editedTitle)
    const bodyBox = await author.findElement(By.name('edit-body'))
    await bodyBox.clear()
    await bodyBox.sendKeys('Body after the edit.')
    const tagsBox = await author.findElement(By.name('edit-tags'))
    await tagsBox.clear()
    await tagsBox.sendKeys('edited')
    await author.findElement(By.xpath("//button[text()='Save changes']")).click()

    await author.wait(until.elementLocated(By.xpath(`//h1[contains(., '${editedTitle}')]`)), 5000)
    bodyText = await author.findElement(By.css('body')).getText()
    assert(bodyText.includes('Body after the edit.'), 'edited body should be shown')
    assert(!bodyText.includes('Body before the edit.'), 'old body should be gone')
    assert(bodyText.includes('(edited)'), 'an edited topic should be marked edited')

    await author.navigate().refresh()
    await author.wait(until.elementLocated(By.xpath(`//h1[contains(., '${editedTitle}')]`)), 5000)
    bodyText = await author.findElement(By.css('body')).getText()
    assert(bodyText.includes('(edited)'), 'the edit should survive a reload')
    assert(bodyText.includes('edited'), 'the new tag should be shown')

    await author.findElement(By.name('comment-body')).sendKeys(originalComment)
    await author.findElement(By.xpath("//button[text()='Post comment']")).click()
    await author.wait(
      until.elementLocated(By.xpath(`//p[contains(., '${originalComment}')]`)),
      5000,
    )

    await author
      .findElement(By.xpath("//button[normalize-space(text())='Edit comment']"))
      .click()
    const commentBox = await author.wait(until.elementLocated(By.name('edit-body')), 5000)
    await commentBox.clear()
    await commentBox.sendKeys(editedComment)
    await author.findElement(By.xpath("//button[text()='Save comment']")).click()
    await author.wait(until.elementLocated(By.xpath(`//p[contains(., '${editedComment}')]`)), 5000)

    bodyText = await author.findElement(By.css('body')).getText()
    assert(bodyText.includes(editedComment), 'edited comment body should be visible')
    assert(!bodyText.includes(originalComment), 'the original comment body should be gone')

    stranger = await buildDriver()
    await stranger.get(`${baseUrl}/register`)
    await stranger.wait(until.elementLocated(By.name('username')), 5000)
    await stranger.findElement(By.name('username')).sendKeys(strangerName)
    await stranger.findElement(By.name('email')).sendKeys(`${strangerName}@example.com`)
    await stranger.findElement(By.name('password')).sendKeys('correcthorse')
    await stranger.findElement(By.css('button[type="submit"]')).click()
    await stranger.wait(until.urlIs(`${baseUrl}/`), 5000)

    await stranger.get(topicUrl)
    await stranger.wait(until.elementLocated(By.name('comment-body')), 5000)
    const strangerText = await stranger.findElement(By.css('body')).getText()
    assert(strangerText.includes(editedTitle), 'stranger should see the edited topic')
    assert(
      !strangerText.includes('Edit topic'),
      'a stranger must not get an edit control on another author topic',
    )
    assert(
      !strangerText.includes('Edit comment'),
      'a stranger must not get an edit control on another author comment',
    )

    console.log('e2e: editing flow passed')
  } finally {
    await author.quit()
    if (stranger) await stranger.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
