import { Builder, By, until } from 'selenium-webdriver'
import chrome from 'selenium-webdriver/chrome.js'
import { publishTopic } from './support.mjs'

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

async function register(driver, username) {
  await driver.get(`${baseUrl}/register`)
  await driver.wait(until.elementLocated(By.name('username')), 5000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('email')).sendKeys(`${username}@example.com`)
  await driver.findElement(By.name('password')).sendKeys('correcthorse')
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(until.urlIs(`${baseUrl}/`), 5000)
}

async function postTopic(driver, section, title, body) {
  await driver.get(`${baseUrl}/s/${section}`)
  await driver.wait(until.elementLocated(By.xpath("//button[text()='New topic']")), 5000)
  await driver.findElement(By.xpath("//button[text()='New topic']")).click()
  await driver.findElement(By.name('title')).sendKeys(title)
  await driver.findElement(By.name('body')).sendKeys(body)
  await driver.findElement(By.xpath("//button[text()='Post']")).click()
  await publishTopic(title)
  await driver.get(`${baseUrl}/s/${section}`)
  await driver.wait(until.elementLocated(By.linkText(title)), 5000)
}

async function activityText(driver) {
  await driver.get(`${baseUrl}/activity`)
  await driver.wait(until.elementLocated(By.css('main')), 5000)
  await driver.sleep(700)
  return (await driver.findElement(By.css('main')).getText()).replace(/\s+/g, ' ')
}

async function run() {
  const driver = await buildDriver()
  const suffix = Date.now().toString(36)
  const username = `e2e_act_a_${suffix}`
  const otherName = `e2e_act_b_${suffix}`
  const generalTitle = `General activity ${suffix}`
  const helpTitle = `Help activity ${suffix}`
  const commentBody = `An activity comment ${suffix}`
  let other
  try {
    await register(driver, username)
    await postTopic(driver, 'general', generalTitle, 'Body for the activity e2e spec.')
    await postTopic(driver, 'help', helpTitle, 'Another body for the activity e2e spec.')

    let text = await activityText(driver)
    assert(text.includes('Activity'), 'the page should be headed Activity')
    assert(text.includes(generalTitle), 'a topic from general should appear')
    assert(text.includes(helpTitle), 'a topic from help should appear')
    assert(
      text.indexOf(helpTitle) < text.indexOf(generalTitle),
      'the newer topic should come first',
    )

    other = await buildDriver()
    await register(other, otherName)
    await other.get(`${baseUrl}/s/general`)
    await other.wait(until.elementLocated(By.linkText(generalTitle)), 5000)
    await other.findElement(By.linkText(generalTitle)).click()
    await other.wait(until.elementLocated(By.name('comment-body')), 5000)
    await other.findElement(By.name('comment-body')).sendKeys(commentBody)
    await other.findElement(By.xpath("//button[text()='Post comment']")).click()
    await other.wait(until.elementLocated(By.xpath(`//p[contains(., '${commentBody}')]`)), 5000)

    text = await activityText(driver)
    assert(
      text.includes(`Comment by ${otherName}`),
      `a new comment should appear in activity, saw: ${text}`,
    )
    assert(text.includes(commentBody), 'the comment excerpt should be shown')
    assert(
      text.indexOf(`Comment by ${otherName}`) < text.indexOf(helpTitle),
      'the newest item should come first',
    )

    const anon = await buildDriver()
    try {
      const anonText = await activityText(anon)
      assert(anonText.includes(generalTitle), 'a signed-out visitor should see activity too')
      assert(
        anonText.includes(`Comment by ${otherName}`),
        'a signed-out visitor should see comments too',
      )
    } finally {
      await anon.quit()
    }

    await driver.get(`${baseUrl}/u/${otherName}`)
    await driver.wait(
      until.elementLocated(By.xpath("//button[normalize-space(.)='Ignore user']")),
      5000,
    )
    await driver.findElement(By.xpath("//button[normalize-space(.)='Ignore user']")).click()
    await driver.wait(
      until.elementLocated(By.xpath("//button[normalize-space(.)='Stop ignoring']")),
      5000,
    )

    text = await activityText(driver)
    assert(
      !text.includes(`Comment by ${otherName}`),
      'an ignored author should drop out of your activity feed',
    )
    assert(text.includes(helpTitle), 'your own topics should still be listed')

    console.log('e2e: activity flow passed')
  } finally {
    await driver.quit()
    if (other) await other.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
